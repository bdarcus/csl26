/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style configuration options.

pub mod bibliography;
pub mod cascade;
pub mod contributors;
pub mod dates;
pub mod integral_name_memory;
pub mod localization;
pub mod locators;
pub mod multilingual;
pub mod processing;
pub mod scoped;
pub mod sorting;
pub mod substitute;
pub mod title_class;

pub use crate::presets::{MultilingualConfigEntry, MultilingualPreset};
pub use bibliography::{
    AnonymousEntriesMode, ArticleJournalBibliographyConfig, ArticleJournalNoPageFallback,
    BibliographyConfig, BibliographyPartitionHeading, BibliographyPartitionKind,
    BibliographyPartitionMode, BibliographySortPartitioning, SubsequentAuthorSubstituteRule,
};
pub use cascade::ScopedRawOptions;
pub use contributors::{
    AndOptions, AndOtherOptions, ContributorConfig, ContributorConfigEntry,
    ContributorSuppressionRule, DelimiterPrecedesLast, DemoteNonDroppingParticle, DisplayAsSort,
    NameForm, RoleLabelDefaults, RoleLabelPresentation, RoleLabelPreset, RoleOptions,
    RoleOptionsEntry, RoleRendering, ShortenListOptions,
};
pub use dates::{DateConfig, DateConfigEntry, DateRangeFormat, NoDateForm};
pub use integral_name_memory::{
    IntegralNameContexts, IntegralNameMemoryConfig, IntegralNameScope, OrgAbbreviationMemoryConfig,
    ResolvedIntegralNameMemoryConfig, ResolvedOrgAbbreviationMemoryConfig, ShortNameDisplay,
    SubsequentNameForm,
};
pub use localization::{Localize, MonthFormat, Scope};
pub use locators::{
    LabelForm, LabelRepeat, LocatorConfig, LocatorConfigEntry, LocatorKindConfig, LocatorPattern,
    LocatorPreset, TypeClass,
};
pub use multilingual::{
    MultilingualConfig, MultilingualMode, MultilingualSegment, MultilingualView,
    PunctuationRealization, PunctuationStyle, PunctuationWidth, RealizationDefault, ScriptConfig,
    SegmentWrap, TermLocale,
};
pub use processing::{
    CitationSortPolicy, Disambiguation, GivennameRule, Group, LabelConfig, LabelParams,
    LabelPreset, Processing, ProcessingBase, ProcessingCustom, RegimeFamily, Sort, SortEntry,
    SortKey, SortSpec,
};
pub use scoped::{
    BibliographyLabelMode, BibliographyLabelWrap, CitationGroupDelimiter, DatePosition, LabelWrap,
    RepeatedAuthorRendering, TitleTerminator,
};
pub use sorting::{SortingConfig, SortingLocale, SortingMultilingualMode};
pub use substitute::{
    Substitute, SubstituteConfig, SubstituteContributor, SubstituteField, SubstituteKey,
    SubstituteTitleQuoteMode,
};

use crate::template::DelimiterPunctuation;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level style configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Style-owned MF2 messages, inherited and merged by message ID.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub messages: HashMap<String, String>,
    /// Substitution rules for missing data. Accepts a preset name (e.g.
    /// "standard") or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_substitute_config",
        default
    )]
    pub substitute: Option<SubstituteConfig>,
    /// Processing mode (author-date, numeric, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<Processing>,
    /// Style-level locale override ID loaded from `locales/overrides/<id>.*`.
    ///
    /// This patches the locale selected by `StyleInfo.default_locale` without
    /// duplicating the full base locale. Runtime loading is limited to the
    /// style-global config; nested citation or bibliography configs are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_override: Option<String>,
    /// Localization settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localize: Option<Localize>,
    /// Multilingual rendering defaults. Accepts a preset name (e.g., `"romanized-translated"`,
    /// `"romanized-only"`) or an explicit configuration block.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_multilingual_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<MultilingualConfigEntry>"))]
    pub multilingual: Option<MultilingualConfig>,
    /// Bibliography sorting policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<SortingConfig>,
    /// Contributor formatting defaults. Accepts a preset name (e.g., "apa")
    /// or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_contributor_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ContributorConfigEntry>"))]
    pub contributors: Option<ContributorConfig>,
    /// Date formatting defaults. Accepts a preset name (e.g., "long")
    /// or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_date_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<DateConfigEntry>"))]
    pub dates: Option<DateConfig>,
    /// Title formatting defaults. Accepts a preset name (e.g., "apa")
    /// or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_titles_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<TitlesConfigEntry>"))]
    pub titles: Option<crate::options::titles::TitlesConfig>,
    /// Locator rendering configuration. Accepts a preset name (e.g., "note")
    /// or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_locator_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<LocatorConfigEntry>"))]
    pub locators: Option<LocatorConfig>,
    /// Page range formatting (expanded, minimal, chicago).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_range_format: Option<PageRangeFormat>,
    /// Separator between page-range endpoints. Overrides the locale's
    /// `page-range-delimiter` (en-dash by default); AMA and similar use `-`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_range_delimiter: Option<String>,
    /// Hyperlink configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<LinksConfig>,
    /// Whether to place periods/commas inside quotation marks.
    /// true = American style ("text."), false = British style ("text".)
    /// Defaults to false; the style must opt in explicitly (embedded styles
    /// that want American-style placement, including `en-US`-locale ones,
    /// set this directly — the locale's own `punctuation-in-quote` grammar
    /// option is not currently consulted by the engine).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub punctuation_in_quote: bool,
    /// Locale-sensitive punctuation-collision overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation: Option<PunctuationConfig>,
    /// Delimiter between volume/issue and pages for serial sources.
    /// Processor adds trailing space when rendering.
    /// Examples: Comma (APA ", "), Colon (Chicago ": ").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_pages_delimiter: Option<DelimiterPunctuation>,
    /// Strip trailing periods from terms, labels, and abbreviated dates.
    #[serde(skip_serializing_if = "Option::is_none", rename = "strip-periods")]
    pub strip_periods: Option<bool>,
    /// Document-level note marker placement and punctuation movement rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<NoteConfig>,
    /// Integral citation name-memory behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integral_name_memory: Option<IntegralNameMemoryConfig>,
    /// Organizational name abbreviation expansion policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_abbreviation_memory: Option<OrgAbbreviationMemoryConfig>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// Citation-local option overrides.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct CitationOptions {
    /// Substitution rules for missing data. Accepts a preset name (e.g.
    /// "standard") or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_substitute_config",
        default
    )]
    pub substitute: Option<SubstituteConfig>,
    /// Processing mode (author-date, numeric, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<Processing>,
    /// Localization settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localize: Option<Localize>,
    /// Multilingual rendering defaults. Accepts a preset name (e.g., `"romanized-translated"`,
    /// `"romanized-only"`) or an explicit configuration block.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_multilingual_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<MultilingualConfigEntry>"))]
    pub multilingual: Option<MultilingualConfig>,
    /// Contributor formatting defaults.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_contributor_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ContributorConfigEntry>"))]
    pub contributors: Option<ContributorConfig>,
    /// Date formatting defaults.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_date_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<DateConfigEntry>"))]
    pub dates: Option<DateConfig>,
    /// Title formatting defaults.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_titles_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<TitlesConfigEntry>"))]
    pub titles: Option<crate::options::titles::TitlesConfig>,
    /// Locator rendering configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_locator_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<LocatorConfigEntry>"))]
    pub locators: Option<LocatorConfig>,
    /// Page range formatting (expanded, minimal, chicago).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_range_format: Option<PageRangeFormat>,
    /// Hyperlink configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<LinksConfig>,
    /// Whether to place periods/commas inside quotation marks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub punctuation_in_quote: bool,
    /// Delimiter between volume/issue and pages for serial sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_pages_delimiter: Option<DelimiterPunctuation>,
    /// Strip trailing periods from terms, labels, and abbreviated dates.
    #[serde(skip_serializing_if = "Option::is_none", rename = "strip-periods")]
    pub strip_periods: Option<bool>,
    /// Document-level note marker placement and punctuation movement rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<NoteConfig>,
    /// Integral citation name-memory behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integral_name_memory: Option<IntegralNameMemoryConfig>,
    /// Organizational name abbreviation expansion policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_abbreviation_memory: Option<OrgAbbreviationMemoryConfig>,
    /// Label wrap policy applied to citation labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_wrap: Option<LabelWrap>,
    /// Delimiter between grouped citation items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_delimiter: Option<CitationGroupDelimiter>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// Bibliography-local option overrides.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct BibliographyOptions {
    /// Substitution rules for missing data. Accepts a preset name (e.g.
    /// "standard") or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_substitute_config",
        default
    )]
    pub substitute: Option<SubstituteConfig>,
    /// Processing mode (author-date, numeric, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<Processing>,
    /// Localization settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localize: Option<Localize>,
    /// Multilingual rendering defaults. Accepts a preset name (e.g., `"romanized-translated"`,
    /// `"romanized-only"`) or an explicit configuration block.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_multilingual_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<MultilingualConfigEntry>"))]
    pub multilingual: Option<MultilingualConfig>,
    /// Bibliography sorting policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<SortingConfig>,
    /// Contributor formatting defaults.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_contributor_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ContributorConfigEntry>"))]
    pub contributors: Option<ContributorConfig>,
    /// Date formatting defaults.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_date_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<DateConfigEntry>"))]
    pub dates: Option<DateConfig>,
    /// Title formatting defaults.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_titles_config",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<TitlesConfigEntry>"))]
    pub titles: Option<crate::options::titles::TitlesConfig>,
    /// Page range formatting (expanded, minimal, chicago).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_range_format: Option<PageRangeFormat>,
    /// Article-journal-specific bibliography policies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_journal: Option<ArticleJournalBibliographyConfig>,
    /// String to substitute for repeating authors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_author_substitute: Option<String>,
    /// Rule for when to apply the substitute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_author_substitute_rule: Option<SubsequentAuthorSubstituteRule>,
    /// Whether to use a hanging indent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hanging_indent: Option<bool>,
    /// Suffix appended to each bibliography entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_suffix: Option<String>,
    /// Separator between bibliography components.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    /// Whether to suppress the trailing period after URLs/DOIs.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suppress_period_after_url: bool,
    /// Force `entry-suffix` even when the entry ends in a URL (MLA).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub entry_suffix_after_url: bool,
    /// Force `entry-suffix` even when the entry ends in a DOI (IEEE).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub entry_suffix_after_doi: bool,
    /// Configuration for compound numeric bibliography entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compound_numeric: Option<bibliography::CompoundNumericConfig>,
    /// Partitioning policy for multilingual bibliography sorting and sections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_partitioning: Option<bibliography::BibliographySortPartitioning>,
    /// Policy for reference-work entries (dictionary/encyclopedia and
    /// dictionary-shaped chapters) with no visible author. See
    /// [`AnonymousEntriesMode`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous_entries: Option<AnonymousEntriesMode>,
    /// Hyperlink configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<LinksConfig>,
    /// Whether to place periods/commas inside quotation marks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub punctuation_in_quote: bool,
    /// Delimiter between volume/issue and pages for serial sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_pages_delimiter: Option<DelimiterPunctuation>,
    /// Bibliography label mode for label-bearing styles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_mode: Option<BibliographyLabelMode>,
    /// Label wrap policy applied to bibliography labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_wrap: Option<BibliographyLabelWrap>,
    /// Placement of issued dates within bibliography entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_position: Option<DatePosition>,
    /// Terminator applied to primary-title bibliography components.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_terminator: Option<TitleTerminator>,
    /// Repeated-author rendering mode for bibliography entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeated_author_rendering: Option<RepeatedAuthorRendering>,
    /// Strip trailing periods from terms, labels, and abbreviated dates.
    #[serde(skip_serializing_if = "Option::is_none", rename = "strip-periods")]
    pub strip_periods: Option<bool>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// Document-level note marker placement rules.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct NoteConfig {
    /// Desired location of movable punctuation relative to closing quotation
    /// marks when note markers are introduced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation: Option<NoteQuotePlacement>,
    /// Desired location of the note marker relative to closing quotation marks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<NoteNumberPlacement>,
    /// Whether the note marker appears before or after the closest movable
    /// punctuation mark.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<NoteMarkerOrder>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// Style-level overrides for locale punctuation-collision defaults.
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct PunctuationConfig {
    /// Policy for a strong terminal mark followed by a style-supplied comma.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong_terminal_comma_policy: Option<StrongTerminalCommaPolicy>,
    /// Terminal marks that suppress a following delimiter's punctuation core.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_suppressing_terminal_marks: Option<String>,
}

impl PunctuationConfig {
    /// Merge `other` over this configuration field by field.
    fn merge(&mut self, other: &Self) {
        if let Some(policy) = other.strong_terminal_comma_policy {
            self.strong_terminal_comma_policy = Some(policy);
        }
        if let Some(marks) = &other.delimiter_suppressing_terminal_marks {
            self.delimiter_suppressing_terminal_marks = Some(marks.clone());
        }
    }
}

/// Controls how a strong terminal mark collides with a following comma.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum StrongTerminalCommaPolicy {
    /// Preserve both the terminal mark and the following comma.
    #[default]
    KeepBoth,
    /// Preserve the terminal mark and suppress the following comma.
    KeepTerminal,
}

/// Controls where movable punctuation is placed relative to closing quotation marks.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NoteQuotePlacement {
    /// Keep movable punctuation inside the closing quotation mark.
    Inside,
    /// Keep movable punctuation outside the closing quotation mark.
    Outside,
    /// Follow org-cite-style adaptive behavior: punctuation stays inside when
    /// it is already flush with the closing quote, otherwise it is placed
    /// outside.
    #[default]
    Adaptive,
}

/// Controls where a footnote number marker is placed relative to closing quotation marks.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NoteNumberPlacement {
    /// Place the note marker inside the closing quotation mark.
    Inside,
    /// Place the note marker outside the closing quotation mark.
    #[default]
    Outside,
    /// Place the note marker on the same side as the movable punctuation when
    /// only one side has punctuation; otherwise default to outside.
    Same,
}

/// Controls whether a note marker appears before or after adjacent movable punctuation.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NoteMarkerOrder {
    /// Place the note marker before the closest movable punctuation mark.
    Before,
    /// Place the note marker after the closest movable punctuation mark.
    #[default]
    After,
}

/// Page range formatting options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PageRangeFormat {
    /// Full expansion: 321-328 → 321–328
    #[default]
    Expanded,
    /// Minimal digits: 321-328 → 321–8
    Minimal,
    /// Minimal two digits: 321-328 → 321–28
    MinimalTwo,
    /// Chicago Manual of Style 15th ed rules
    Chicago,
    /// Chicago Manual of Style 16th/17th ed rules
    Chicago16,
}

pub mod titles;

pub use title_class::{
    TitleCategory, classified_ref_types, container_title_category, parent_serial_title_category,
    title_category,
};
pub use titles::{TextCase, TitleRendering, TitlesConfig, TitlesConfigEntry};

/// Structured link options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct LinksConfig {
    /// Link value to the item's DOI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<bool>,
    /// Link value to the item's URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<bool>,
    /// The target for the link (url, doi, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<LinkTarget>,
    /// What text should be hyperlinked (title, url, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<LinkAnchor>,
    /// Omit the URL scheme (e.g. `http://`, `https://`) when rendering a link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_protocol: Option<bool>,
}

/// Link target options.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LinkTarget {
    Url,
    Doi,
    UrlOrDoi,
    Pubmed,
    Pmcid,
}

/// Link anchor options.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LinkAnchor {
    /// Link the title component.
    Title,
    /// Link the URL component itself.
    Url,
    /// Link the DOI component itself.
    Doi,
    /// Link the specific component this config is attached to.
    Component,
    /// Link the entire bibliography entry.
    Entry,
}

impl Config {
    /// Merge `other`'s multilingual block field-wise rather than replacing it.
    ///
    /// Replacing the whole block means a style that extends a parent and sets
    /// any single multilingual key silently discards every inherited one — see
    /// [`MultilingualConfig::merge`] and bean `csl26-p7kj`.
    fn merge_multilingual(&mut self, other: &Config) {
        let Some(other_multilingual) = &other.multilingual else {
            return;
        };
        if let Some(multilingual) = &mut self.multilingual {
            multilingual.merge(other_multilingual);
        } else {
            self.multilingual = Some(other_multilingual.clone());
        }
    }

    fn merge_punctuation(&mut self, other: &Config) {
        let Some(other_punctuation) = &other.punctuation else {
            return;
        };
        if let Some(punctuation) = &mut self.punctuation {
            punctuation.merge(other_punctuation);
        } else {
            self.punctuation = Some(other_punctuation.clone());
        }
    }

    /// Effective processing mode, falling back to the default when unset.
    ///
    /// Centralizes the `processing: None` fallback so every consumer resolves
    /// the same default (`Processing::default()`) instead of hardcoding it.
    pub fn effective_processing(&self) -> Processing {
        self.processing.clone().unwrap_or_default()
    }

    /// Merge another config into this one, with `other` taking precedence.
    ///
    /// Used for combining global options with context-specific (citation/bibliography) options.
    /// Only non-None fields from `other` override fields in `self`.
    pub fn merge(&mut self, other: &Config) {
        crate::merge_options!(
            self,
            other,
            processing,
            locale_override,
            localize,
            dates,
            titles,
            locators,
            page_range_format,
            page_range_delimiter,
            links,
            volume_pages_delimiter,
            locale_override,
            strip_periods,
            notes,
            integral_name_memory,
            org_abbreviation_memory,
            custom,
        );

        self.merge_multilingual(other);
        self.merge_punctuation(other);
        self.messages.extend(other.messages.clone());

        if let Some(other_sorting) = &other.sorting {
            if let Some(this_sorting) = &mut self.sorting {
                this_sorting.merge(other_sorting);
            } else {
                self.sorting = Some(other_sorting.clone());
            }
        }

        if let Some(other_substitute) = &other.substitute {
            if let Some(this_substitute) = &self.substitute {
                self.substitute = Some(SubstituteConfig::merged(this_substitute, other_substitute));
            } else {
                self.substitute = Some(other_substitute.clone());
            }
        }

        if let Some(other_contributors) = &other.contributors {
            if let Some(this_contributors) = &mut self.contributors {
                this_contributors.merge(other_contributors);
            } else {
                self.contributors = Some(other_contributors.clone());
            }
        }

        if other.punctuation_in_quote {
            self.punctuation_in_quote = true;
        }
    }

    /// Create a merged config from base and override, returning a new Config.
    ///
    /// Convenience method that clones base, then merges override into it.
    pub fn merged(base: &Config, override_config: &Config) -> Config {
        let mut result = base.clone();
        result.merge(override_config);
        result
    }
}

impl CitationOptions {
    /// Convert citation-local overrides into the runtime config shape.
    #[must_use]
    pub fn to_config(&self) -> Config {
        Config {
            messages: HashMap::new(),
            substitute: self.substitute.clone(),
            processing: self.processing.clone(),
            locale_override: None,
            localize: self.localize.clone(),
            multilingual: self.multilingual.clone(),
            sorting: None,
            contributors: self.contributors.clone(),
            dates: self.dates.clone(),
            titles: self.titles.clone(),
            locators: self.locators.clone(),
            page_range_format: self.page_range_format.clone(),
            page_range_delimiter: None,
            links: self.links.clone(),
            punctuation_in_quote: self.punctuation_in_quote,
            punctuation: None,
            volume_pages_delimiter: self.volume_pages_delimiter.clone(),
            strip_periods: self.strip_periods,
            notes: self.notes.clone(),
            integral_name_memory: self.integral_name_memory.clone(),
            org_abbreviation_memory: self.org_abbreviation_memory.clone(),
            custom: self.custom.clone(),
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }

    /// Merge citation-local overrides over a base config.
    #[must_use]
    pub fn merged_with(&self, base: &Config) -> Config {
        Config::merged(base, &self.to_config())
    }

    /// Merge citation-local overrides over a base config, merging nested
    /// option blocks field-by-field when a trustworthy authored scope mapping
    /// is available (see [`cascade::ScopedRawOptions`]).
    ///
    /// `raw_options` is the chain-merged authored `citation.options` mapping
    /// carried on the resolved [`crate::Style`]. When it is absent or no
    /// longer round-trips to `self` (post-parse mutation), this behaves
    /// exactly like [`CitationOptions::merged_with`].
    #[must_use]
    pub fn merged_with_raw(
        &self,
        base: &Config,
        raw_options: Option<&serde_yaml::Value>,
    ) -> Config {
        let mut merged = self.merged_with(base);
        if let Some(raw) = raw_options
            && cascade::authored_matches(raw, self)
        {
            cascade::merge_citation_config_blocks_from_raw(&mut merged, base, raw);
        }
        merged
    }

    /// Merge `other` into `self`, with `other` taking precedence for each field.
    pub fn merge(&mut self, other: &CitationOptions) {
        crate::merge_options!(
            self,
            other,
            processing,
            localize,
            multilingual,
            dates,
            titles,
            locators,
            page_range_format,
            links,
            volume_pages_delimiter,
            strip_periods,
            notes,
            integral_name_memory,
            org_abbreviation_memory,
            label_wrap,
            group_delimiter,
            custom,
        );

        if let Some(other_substitute) = &other.substitute {
            if let Some(this_substitute) = &self.substitute {
                self.substitute = Some(SubstituteConfig::merged(this_substitute, other_substitute));
            } else {
                self.substitute = Some(other_substitute.clone());
            }
        }

        if let Some(other_contributors) = &other.contributors {
            if let Some(this_contributors) = &mut self.contributors {
                this_contributors.merge(other_contributors);
            } else {
                self.contributors = Some(other_contributors.clone());
            }
        }

        if other.punctuation_in_quote {
            self.punctuation_in_quote = true;
        }
    }
}

impl BibliographyOptions {
    /// Convert bibliography-entry overrides into bibliography-only runtime config.
    #[must_use]
    pub fn to_bibliography_config(&self) -> BibliographyConfig {
        BibliographyConfig {
            article_journal: self.article_journal.clone(),
            subsequent_author_substitute: self.subsequent_author_substitute.clone(),
            subsequent_author_substitute_rule: self.subsequent_author_substitute_rule.clone(),
            hanging_indent: self.hanging_indent,
            entry_suffix: self.entry_suffix.clone(),
            separator: self.separator.clone(),
            suppress_period_after_url: self.suppress_period_after_url,
            entry_suffix_after_url: self.entry_suffix_after_url,
            entry_suffix_after_doi: self.entry_suffix_after_doi,
            custom: None,
            compound_numeric: self.compound_numeric.clone(),
            sort_partitioning: self.sort_partitioning.clone(),
            anonymous_entries: self.anonymous_entries,
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }

    /// Convert bibliography-local overrides into the runtime config shape.
    #[must_use]
    pub fn to_config(&self) -> Config {
        Config {
            messages: HashMap::new(),
            substitute: self.substitute.clone(),
            processing: self.processing.clone(),
            locale_override: None,
            localize: self.localize.clone(),
            multilingual: self.multilingual.clone(),
            sorting: self.sorting.clone(),
            contributors: self.contributors.clone(),
            dates: self.dates.clone(),
            titles: self.titles.clone(),
            locators: None,
            page_range_format: self.page_range_format.clone(),
            page_range_delimiter: None,
            links: self.links.clone(),
            punctuation_in_quote: self.punctuation_in_quote,
            punctuation: None,
            volume_pages_delimiter: self.volume_pages_delimiter.clone(),
            strip_periods: self.strip_periods,
            notes: None,
            integral_name_memory: None,
            org_abbreviation_memory: None,
            custom: self.custom.clone(),
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }

    /// Merge bibliography-local overrides over a base config.
    #[must_use]
    pub fn merged_with(&self, base: &Config) -> Config {
        Config::merged(base, &self.to_config())
    }

    /// Merge bibliography-local overrides over a base config, merging nested
    /// option blocks field-by-field when a trustworthy authored scope mapping
    /// is available (see [`cascade::ScopedRawOptions`]).
    ///
    /// `raw_options` is the chain-merged authored `bibliography.options`
    /// mapping carried on the resolved [`crate::Style`]. When it is absent or
    /// no longer round-trips to `self` (post-parse mutation), this behaves
    /// exactly like [`BibliographyOptions::merged_with`].
    #[must_use]
    pub fn merged_with_raw(
        &self,
        base: &Config,
        raw_options: Option<&serde_yaml::Value>,
    ) -> Config {
        let mut merged = self.merged_with(base);
        if let Some(raw) = raw_options
            && cascade::authored_matches(raw, self)
        {
            cascade::merge_bibliography_config_blocks_from_raw(&mut merged, base, raw);
        }
        merged
    }

    /// Merge `other` into `self`, with `other` taking precedence for each field.
    pub fn merge(&mut self, other: &BibliographyOptions) {
        crate::merge_options!(
            self,
            other,
            processing,
            localize,
            multilingual,
            dates,
            titles,
            page_range_format,
            links,
            volume_pages_delimiter,
            strip_periods,
            article_journal,
            subsequent_author_substitute,
            subsequent_author_substitute_rule,
            hanging_indent,
            entry_suffix,
            separator,
            compound_numeric,
            sort_partitioning,
            anonymous_entries,
            label_mode,
            label_wrap,
            date_position,
            title_terminator,
            repeated_author_rendering,
            custom,
        );

        self.merge_shared_fields(other);
    }

    fn merge_shared_fields(&mut self, other: &BibliographyOptions) {
        if let Some(other_sorting) = &other.sorting {
            if let Some(this_sorting) = &mut self.sorting {
                this_sorting.merge(other_sorting);
            } else {
                self.sorting = Some(other_sorting.clone());
            }
        }

        if let Some(other_substitute) = &other.substitute {
            if let Some(this_substitute) = &self.substitute {
                self.substitute = Some(SubstituteConfig::merged(this_substitute, other_substitute));
            } else {
                self.substitute = Some(other_substitute.clone());
            }
        }

        if let Some(other_contributors) = &other.contributors {
            if let Some(this_contributors) = &mut self.contributors {
                this_contributors.merge(other_contributors);
            } else {
                self.contributors = Some(other_contributors.clone());
            }
        }

        if other.punctuation_in_quote {
            self.punctuation_in_quote = true;
        }
        if other.suppress_period_after_url {
            self.suppress_period_after_url = true;
        }
        if other.entry_suffix_after_url {
            self.entry_suffix_after_url = true;
        }
        if other.entry_suffix_after_doi {
            self.entry_suffix_after_doi = true;
        }

        for (key, value) in &other.unknown_fields {
            self.unknown_fields.insert(key.clone(), value.clone());
        }
    }
}

/// Deserialize contributor config from either a preset name or explicit config.
fn deserialize_contributor_config<'de, D>(
    deserializer: D,
) -> Result<Option<ContributorConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<ContributorConfigEntry> = Option::deserialize(deserializer)?;
    Ok(value.map(|entry| entry.resolve()))
}

/// Deserialize date config from either a preset name or explicit config.
fn deserialize_date_config<'de, D>(deserializer: D) -> Result<Option<DateConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<DateConfigEntry> = Option::deserialize(deserializer)?;
    Ok(value.map(|entry| entry.resolve()))
}

/// Deserialize titles config from either a preset name or explicit config.
fn deserialize_titles_config<'de, D>(
    deserializer: D,
) -> Result<Option<crate::options::titles::TitlesConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<crate::options::titles::TitlesConfigEntry> =
        Option::deserialize(deserializer)?;
    Ok(value.map(|entry| entry.resolve()))
}

/// Deserialize locator config from either a preset name or explicit config.
fn deserialize_locator_config<'de, D>(deserializer: D) -> Result<Option<LocatorConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<LocatorConfigEntry> = Option::deserialize(deserializer)?;
    Ok(value.map(|entry| entry.resolve()))
}

/// Deserialize substitute config from either a preset name or explicit config.
///
/// Eagerly resolves a `Preset` variant to its `Explicit` form (mirroring
/// `deserialize_contributor_config`/`deserialize_date_config`/etc.), so the
/// typed value always serializes as a mapping. Without this, the `extends`
/// overlay's raw-YAML deep merge (`style/overlay.rs`) sees a preset-name
/// scalar for an authored `substitute: <preset>` override and whole-replaces
/// the inherited `Substitute` block instead of field-merging it, silently
/// dropping fields the preset doesn't set (e.g. `role-substitute`).
fn deserialize_substitute_config<'de, D>(
    deserializer: D,
) -> Result<Option<SubstituteConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<SubstituteConfig> = Option::deserialize(deserializer)?;
    Ok(value.map(|config| match config {
        SubstituteConfig::Preset(preset) => SubstituteConfig::Explicit(preset.config()),
        explicit @ SubstituteConfig::Explicit(_) => explicit,
    }))
}

/// Deserialize multilingual config from either a preset name or an explicit block.
fn deserialize_multilingual_config<'de, D>(
    deserializer: D,
) -> Result<Option<MultilingualConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<crate::presets::MultilingualConfigEntry> = Option::deserialize(deserializer)?;
    Ok(value.map(|entry| entry.resolve()))
}

impl<'de> Deserialize<'de> for Config {
    #[allow(
        clippy::too_many_lines,
        reason = "the local wire type intentionally mirrors the complete public config"
    )]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct ConfigWire {
            #[serde(default)]
            messages: HashMap<String, String>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                deserialize_with = "deserialize_substitute_config",
                default
            )]
            substitute: Option<SubstituteConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            processing: Option<Processing>,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale_override: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            localize: Option<Localize>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                deserialize_with = "deserialize_multilingual_config",
                default
            )]
            multilingual: Option<MultilingualConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            sorting: Option<SortingConfig>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                deserialize_with = "deserialize_contributor_config",
                default
            )]
            contributors: Option<ContributorConfig>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                deserialize_with = "deserialize_date_config",
                default
            )]
            dates: Option<DateConfig>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                deserialize_with = "deserialize_titles_config",
                default
            )]
            titles: Option<crate::options::titles::TitlesConfig>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                deserialize_with = "deserialize_locator_config",
                default
            )]
            locators: Option<LocatorConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            page_range_format: Option<PageRangeFormat>,
            #[serde(skip_serializing_if = "Option::is_none")]
            page_range_delimiter: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            links: Option<LinksConfig>,
            #[serde(default, skip_serializing_if = "std::ops::Not::not")]
            punctuation_in_quote: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            punctuation: Option<PunctuationConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            volume_pages_delimiter: Option<DelimiterPunctuation>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "strip-periods")]
            strip_periods: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            notes: Option<NoteConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            integral_name_memory: Option<IntegralNameMemoryConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            org_abbreviation_memory: Option<OrgAbbreviationMemoryConfig>,
            #[serde(default)]
            profile: Option<serde_yaml::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            custom: Option<HashMap<String, serde_json::Value>>,
            #[serde(flatten)]
            unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
        }

        let wire = ConfigWire::deserialize(deserializer)?;
        if wire.profile.is_some() {
            return Err(serde::de::Error::custom(
                "`options.profile` was removed; use `options.contributors`, `citation.options.label-wrap`, `citation.options.group-delimiter`, `bibliography.options.label-mode`, `bibliography.options.label-wrap`, `bibliography.options.date-position`, `bibliography.options.title-terminator`, `bibliography.options.repeated-author-rendering`, or `bibliography.options.volume-pages-delimiter`",
            ));
        }

        Ok(Self {
            messages: wire.messages,
            substitute: wire.substitute,
            processing: wire.processing,
            locale_override: wire.locale_override,
            localize: wire.localize,
            multilingual: wire.multilingual,
            sorting: wire.sorting,
            contributors: wire.contributors,
            dates: wire.dates,
            titles: wire.titles,
            locators: wire.locators,
            page_range_format: wire.page_range_format,
            page_range_delimiter: wire.page_range_delimiter,
            links: wire.links,
            punctuation_in_quote: wire.punctuation_in_quote,
            punctuation: wire.punctuation,
            volume_pages_delimiter: wire.volume_pages_delimiter,
            strip_periods: wire.strip_periods,
            notes: wire.notes,
            integral_name_memory: wire.integral_name_memory,
            org_abbreviation_memory: wire.org_abbreviation_memory,
            custom: wire.custom,
            unknown_fields: wire.unknown_fields,
        })
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
    use rstest::rstest;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.substitute.is_none());
        assert!(config.processing.is_none());
    }

    #[test]
    fn test_author_date_processing() {
        let processing = Processing::AuthorDate;
        let config = processing.config();
        let disambiguate = config.disambiguate.unwrap();
        assert!(disambiguate.year_suffix);
        assert!(!disambiguate.names);
        assert!(!disambiguate.add_givenname);
        assert_eq!(
            processing.default_bibliography_sort(),
            Some(crate::presets::SortPreset::AuthorDateTitle)
        );
        assert_eq!(
            config.sort,
            Some(SortEntry::Preset(
                crate::presets::SortPreset::AuthorDateTitle
            ))
        );
    }

    #[test]
    fn test_processing_default_bibliography_sorts() {
        assert_eq!(Processing::Numeric.default_bibliography_sort(), None);
        assert_eq!(
            Processing::Note.default_bibliography_sort(),
            Some(crate::presets::SortPreset::AuthorTitleDate)
        );
        assert_eq!(
            Processing::Label(LabelConfig::default()).default_bibliography_sort(),
            Some(crate::presets::SortPreset::AuthorDateTitle)
        );
    }

    #[test]
    fn test_processing_default_citation_sort_policy_is_explicit_only() {
        assert_eq!(
            Processing::AuthorDate.default_citation_sort_policy(),
            CitationSortPolicy::ExplicitOnly
        );
        assert_eq!(
            Processing::Note.default_citation_sort_policy(),
            CitationSortPolicy::ExplicitOnly
        );
    }

    #[test]
    fn test_substitute_default() {
        let sub = Substitute::default();
        assert_eq!(sub.template.len(), 3);
    }

    #[test]
    fn test_config_yaml_roundtrip() {
        let yaml = r#"
substitute:
  contributor-role-form: short
  template:
    - editor
    - title
processing: author-date
contributors:
  display-as-sort: first
  and: symbol
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.substitute.is_some());
        assert_eq!(config.processing, Some(Processing::AuthorDate));
        assert_eq!(
            config.contributors.as_ref().unwrap().and,
            Some(AndOptions::Symbol)
        );
    }

    #[test]
    fn test_sorting_config_deserializes_and_roundtrips() {
        let yaml = r#"
sorting:
  locale: sv-SE
  multilingual: romanized
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let sorting = config.sorting.as_ref().expect("sorting should parse");

        assert_eq!(
            sorting.locale,
            Some(SortingLocale::Bcp47("sv-SE".to_string()))
        );
        assert_eq!(
            sorting.multilingual,
            Some(SortingMultilingualMode::Romanized)
        );

        let serialized = serde_yaml::to_string(&config).unwrap();
        let reparsed: Config = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(reparsed.sorting, config.sorting);
    }

    #[test]
    fn test_sorting_config_defaults_and_unknown_fields() {
        let yaml = r#"
sorting:
  future-key: true
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let sorting = config.sorting.as_ref().expect("sorting should parse");

        assert_eq!(sorting.effective_locale(), SortingLocale::Auto);
        assert_eq!(
            sorting.effective_multilingual(),
            SortingMultilingualMode::Uniform
        );
        assert!(sorting.unknown_fields.contains_key("future-key"));
    }

    #[test]
    fn test_bibliography_sorting_override_merges_partially() {
        let base: Config = serde_yaml::from_str(
            r#"
sorting:
  locale: de-DE
  multilingual: uniform
"#,
        )
        .unwrap();
        let bib: BibliographyOptions = serde_yaml::from_str(
            r#"
sorting:
  multilingual: romanized
"#,
        )
        .unwrap();

        let merged = bib.merged_with(&base);
        let sorting = merged.sorting.expect("merged sorting should exist");
        assert_eq!(
            sorting.locale,
            Some(SortingLocale::Bcp47("de-DE".to_string()))
        );
        assert_eq!(
            sorting.multilingual,
            Some(SortingMultilingualMode::Romanized)
        );
    }

    #[test]
    fn test_contributor_config_preset() {
        // Test that a preset name parses and resolves correctly for contributors
        let yaml = r#"contributors: apa"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let contributors = config.contributors.unwrap();
        assert_eq!(contributors.and, Some(AndOptions::Symbol));
        assert_eq!(contributors.display_as_sort, Some(DisplayAsSort::First));
    }

    #[test]
    fn test_role_label_presets_parse_and_resolve_precedence() {
        let yaml = r#"
contributors:
  role:
    preset: short-suffix
    roles:
      editor:
        preset: long-suffix
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let contributors = config.contributors.unwrap();

        assert_eq!(
            contributors.effective_role_label_preset(&crate::template::ContributorRole::Editor),
            Some(RoleLabelPreset::LongSuffix)
        );
        assert_eq!(
            contributors.effective_role_label_preset(&crate::template::ContributorRole::Translator),
            Some(RoleLabelPreset::ShortSuffix)
        );

        // Scalar shorthand form — must parse identically for preset-only case
        let yaml_scalar = r#"
contributors:
  role: short-suffix
"#;
        let config2: Config = serde_yaml::from_str(yaml_scalar).unwrap();
        let contributors2 = config2.contributors.unwrap();

        assert_eq!(
            contributors2
                .effective_role_label_preset(&crate::template::ContributorRole::Translator),
            Some(RoleLabelPreset::ShortSuffix)
        );
    }

    #[test]
    fn test_role_specific_name_order_override_is_available() {
        let yaml = r#"
contributors:
  role:
    roles:
      translator:
        name-order: given-first
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let contributors = config.contributors.unwrap();

        assert_eq!(
            contributors.effective_role_name_order(&crate::template::ContributorRole::Translator),
            Some(&crate::template::NameOrder::GivenFirst)
        );
    }

    #[test]
    fn test_date_config_preset() {
        // Test that a preset name parses and resolves correctly for dates
        let yaml = r#"dates: long"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let dates = config.dates.unwrap();
        assert_eq!(dates.month, MonthFormat::Long);
    }

    #[test]
    fn test_titles_config_preset() {
        // Test that a preset name parses and resolves correctly for titles
        let yaml = r#"titles: chicago"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let titles = config.titles.unwrap();
        assert_eq!(titles.component.unwrap().quote, Some(true));
        assert_eq!(titles.monograph.unwrap().emph, Some(true));
    }

    #[test]
    fn test_substitute_config_preset() {
        // Test that a preset name parses correctly
        let yaml = r#"substitute: standard"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.substitute.is_some());
        let resolved = config.substitute.unwrap().resolve();
        assert_eq!(resolved.template.len(), 3);
        assert_eq!(resolved.template[0], SubstituteKey::Editor);
    }

    #[test]
    fn test_substitute_config_explicit() {
        // Test that explicit config still works
        let yaml = r#"
substitute:
  template:
    - title
    - editor
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let resolved = config.substitute.unwrap().resolve();
        assert_eq!(resolved.template[0], SubstituteKey::Title);
        assert_eq!(resolved.template[1], SubstituteKey::Editor);
    }

    #[test]
    fn test_config_merge_precedence() {
        // Base config with global options
        let base_yaml = r#"
processing: author-date
locale-override: en-US-base
contributors:
  display-as-sort: first
  and: symbol
"#;
        let mut base: Config = serde_yaml::from_str(base_yaml).unwrap();

        // Override config (e.g., citation-specific options)
        let override_yaml = r#"
contributors:
  and: text
locale-override: en-US-chicago
"#;
        let override_config: Config = serde_yaml::from_str(override_yaml).unwrap();

        // Merge: override takes precedence
        base.merge(&override_config);

        // Processing should remain from base (not overridden)
        assert_eq!(base.processing, Some(Processing::AuthorDate));
        assert_eq!(base.locale_override.as_deref(), Some("en-US-chicago"));

        // Contributors should be merged with override values taking precedence
        assert_eq!(
            base.contributors.as_ref().unwrap().and,
            Some(AndOptions::Text)
        );
    }

    #[test]
    fn test_config_deserializes_locale_override() {
        let config: Config = serde_yaml::from_str("locale-override: en-US-chicago").unwrap();
        assert_eq!(config.locale_override.as_deref(), Some("en-US-chicago"));
    }

    #[test]
    fn test_config_merged_convenience() {
        let base = Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        };
        let override_config = Config {
            punctuation_in_quote: true,
            ..Default::default()
        };

        let merged = Config::merged(&base, &override_config);

        // Both fields preserved
        assert_eq!(merged.processing, Some(Processing::AuthorDate));
        assert!(merged.punctuation_in_quote);
    }

    #[test]
    fn test_citation_options_merge_overrides_citation_fields_only() {
        let base = Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        };

        let overrides = CitationOptions {
            strip_periods: Some(true),
            locators: Some(LocatorConfig::default()),
            ..Default::default()
        };

        let merged = overrides.merged_with(&base);
        assert_eq!(merged.processing, Some(Processing::AuthorDate));
        assert!(merged.strip_periods.unwrap_or(false));
        assert!(merged.locators.is_some());
    }

    #[test]
    fn test_punctuation_config_deserializes_and_merges_field_by_field() {
        let yaml = r#"
punctuation:
  strong-terminal-comma-policy: keep-terminal
  delimiter-suppressing-terminal-marks: "?!…"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let punctuation = config.punctuation.as_ref().unwrap();
        assert_eq!(
            punctuation.strong_terminal_comma_policy,
            Some(StrongTerminalCommaPolicy::KeepTerminal)
        );
        assert_eq!(
            punctuation.delimiter_suppressing_terminal_marks.as_deref(),
            Some("?!…")
        );

        let override_config = Config {
            punctuation: Some(PunctuationConfig {
                strong_terminal_comma_policy: Some(StrongTerminalCommaPolicy::KeepBoth),
                delimiter_suppressing_terminal_marks: None,
            }),
            ..Default::default()
        };
        let merged = Config::merged(&config, &override_config);
        let punctuation = merged.punctuation.as_ref().unwrap();
        assert_eq!(
            punctuation.strong_terminal_comma_policy,
            Some(StrongTerminalCommaPolicy::KeepBoth)
        );
        assert_eq!(
            punctuation.delimiter_suppressing_terminal_marks.as_deref(),
            Some("?!…")
        );
    }

    #[test]
    fn test_bibliography_options_merge_projects_shared_fields_only() {
        let base = Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        };

        let overrides = BibliographyOptions {
            entry_suffix: Some(".".into()),
            separator: Some(", ".to_string()),
            suppress_period_after_url: true,
            ..Default::default()
        };

        let merged = overrides.merged_with(&base);
        assert_eq!(merged.processing, Some(Processing::AuthorDate));
        assert!(merged.locators.is_none());
        assert!(merged.notes.is_none());
        let bibliography = overrides.to_bibliography_config();
        assert_eq!(bibliography.entry_suffix.as_deref(), Some("."));
        assert_eq!(bibliography.separator.as_deref(), Some(", "));
        assert!(bibliography.suppress_period_after_url);
    }

    #[test]
    fn test_bibliography_options_merge_leaves_shared_base_when_only_shared_overrides_exist() {
        let base = Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        };

        let overrides = BibliographyOptions {
            contributors: Some(ContributorConfig::default()),
            ..Default::default()
        };

        let merged = overrides.merged_with(&base);
        assert_eq!(merged.processing, Some(Processing::AuthorDate));
        assert!(merged.contributors.is_some());
    }

    #[test]
    fn citation_options_captures_unknown_fields_for_forward_compat() {
        let yaml = "future-key: true\n";
        let opts: CitationOptions = serde_yaml::from_str(yaml).unwrap();
        assert!(opts.unknown_fields.contains_key("future-key"));
    }

    #[test]
    fn bibliography_options_captures_unknown_fields_for_forward_compat() {
        let yaml = "future-key: true\n";
        let opts: BibliographyOptions = serde_yaml::from_str(yaml).unwrap();
        assert!(opts.unknown_fields.contains_key("future-key"));
    }

    #[test]
    fn note_config_captures_unknown_fields_for_forward_compat() {
        let yaml = "punctuation: inside\nfuture-key: true\n";
        let cfg: NoteConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.unknown_fields.contains_key("future-key"));
        assert_eq!(cfg.punctuation, Some(NoteQuotePlacement::Inside));
    }

    #[test]
    fn test_multilingual_preset_romanized_translated_parses_and_resolves() {
        // `romanized-translated` resolves to Combined title mode (romanized [translated])
        let yaml = r#"multilingual: romanized-translated"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let ml = config.multilingual.unwrap();
        assert_eq!(ml.title_mode, Some(MultilingualMode::Combined));
        assert_eq!(ml.name_mode, Some(MultilingualMode::Transliterated));
        assert_eq!(ml.preferred_script.as_deref(), Some("Latn"));
    }

    #[test]
    fn test_multilingual_preset_romanized_only_parses_and_resolves() {
        // `romanized-only` resolves to Transliterated title mode (no translation)
        let yaml = r#"multilingual: romanized-only"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let ml = config.multilingual.unwrap();
        assert_eq!(ml.title_mode, Some(MultilingualMode::Transliterated));
        assert_eq!(ml.name_mode, Some(MultilingualMode::Transliterated));
        assert_eq!(ml.preferred_script.as_deref(), Some("Latn"));
    }

    #[test]
    fn test_multilingual_preset_romanized_script_translated_parses_and_resolves() {
        // `romanized-script-translated` resolves to a Pattern title mode (romanized original-script [translated])
        // and Pattern name mode (romanized original-script), with Latn script and CJK native ordering.
        use crate::options::multilingual::{MultilingualSegment, MultilingualView, SegmentWrap};
        let yaml = r#"multilingual: romanized-script-translated"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let ml = config.multilingual.unwrap();
        assert_eq!(
            ml.title_mode,
            Some(MultilingualMode::Pattern(vec![
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
            ]))
        );
        assert_eq!(
            ml.name_mode,
            Some(MultilingualMode::Pattern(vec![
                MultilingualSegment {
                    view: MultilingualView::Transliterated,
                    wrap: SegmentWrap::None,
                },
                MultilingualSegment {
                    view: MultilingualView::OriginalScript,
                    wrap: SegmentWrap::None,
                },
            ]))
        );
        assert_eq!(ml.preferred_script.as_deref(), Some("Latn"));
        assert!(ml.scripts.get("Han").is_some_and(|s| s.use_native_ordering));
        assert!(
            ml.scripts
                .get("Hangul")
                .is_some_and(|s| s.use_native_ordering)
        );
    }

    #[test]
    fn test_multilingual_explicit_block_transliterated_roundtrips() {
        // Verify a Transliterated explicit block survives YAML serialize→deserialize.
        let yaml = r#"
multilingual:
  title-mode: transliterated
  preferred-script: Latn
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let ml = config.multilingual.clone().unwrap();
        assert_eq!(ml.title_mode, Some(MultilingualMode::Transliterated));
        assert_eq!(ml.preferred_script.as_deref(), Some("Latn"));

        let yaml2 = serde_yaml::to_string(&config).unwrap();
        let config2: Config = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(config2.multilingual, config.multilingual);
    }

    #[test]
    fn test_multilingual_pattern_block_roundtrips() {
        // Exercises the externally-tagged `{pattern: [...]}` YAML path — the case that
        // breaks under serde_yaml's untagged+enum limitation without the custom Deserialize.
        let yaml = r#"
multilingual:
  title-mode:
    pattern:
      - view: original-script
      - view: translated
        wrap: brackets
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let ml = config.multilingual.clone().unwrap();
        assert!(
            matches!(ml.title_mode, Some(MultilingualMode::Pattern(_))),
            "expected Pattern mode, got {:?}",
            ml.title_mode
        );

        let yaml2 = serde_yaml::to_string(&config).unwrap();
        let config2: Config = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(config2.multilingual, config.multilingual);
    }

    /// An overlay that names one multilingual key must not discard the rest of
    /// the inherited block. Regression for bean `csl26-p7kj`: `multilingual`
    /// was merged whole-value, so a style extending `gb-t-7714-2025-numeric`
    /// and adding an unrelated `scripts` entry silently dropped the inherited
    /// `punctuation-width: mixed`, reverting Chinese punctuation to half-width.
    #[rstest]
    #[case::scripts_overlay(
        "multilingual:\n  scripts:\n    Hang:\n      use-native-ordering: true\n",
        "Hang"
    )]
    #[case::term_locale_overlay("multilingual:\n  term-locale: item\n", "Hani")]
    fn given_partial_multilingual_overlay_when_merging_then_inherited_fields_survive(
        #[case] overlay_yaml: &str,
        #[case] expected_script_key: &str,
    ) {
        // given: a parent declaring several multilingual fields at once
        let base_yaml = "\
multilingual:
  punctuation-width: mixed
  preferred-script: Latn
  scripts:
    Hani:
      use-native-ordering: true
";
        let mut base: Config = serde_yaml::from_str(base_yaml).unwrap();

        // when: an overlay names only one of them
        let overlay: Config = serde_yaml::from_str(overlay_yaml).unwrap();
        base.merge(&overlay);

        // then: the inherited fields the overlay never mentioned are preserved
        let merged = base.multilingual.expect("merged multilingual block");
        assert_eq!(
            merged.punctuation_width,
            Some(PunctuationWidth::Mixed),
            "inherited punctuation-width must survive a partial overlay"
        );
        assert_eq!(merged.preferred_script.as_deref(), Some("Latn"));
        assert!(
            merged.scripts.contains_key(expected_script_key),
            "expected scripts key {expected_script_key:?}, got {:?}",
            merged.scripts.keys().collect::<Vec<_>>()
        );
    }

    /// An overlay that does name a field still wins over the inherited value.
    #[test]
    fn given_multilingual_overlay_setting_a_field_when_merging_then_overlay_wins() {
        let mut base: Config =
            serde_yaml::from_str("multilingual:\n  punctuation-width: mixed\n").unwrap();
        let overlay: Config =
            serde_yaml::from_str("multilingual:\n  punctuation-width: bylan\n").unwrap();

        base.merge(&overlay);

        assert_eq!(
            base.multilingual.unwrap().punctuation_width,
            Some(PunctuationWidth::Bylan)
        );
    }
}
