/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Style configuration options.
//!
//! This module defines the configuration groups and options available in CSLN styles.
//! Much of the logic that CSL 1.0 handles in procedural template conditionals is
//! instead configured declaratively here.

use crate::template::DelimiterPunctuation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level style configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Substitution rules for missing data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substitute: Option<Substitute>,
    /// Processing mode (author-date, numeric, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<Processing>,
    /// Localization settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localize: Option<Localize>,
    /// Contributor formatting defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<ContributorConfig>,
    /// Date formatting defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dates: Option<DateConfig>,
    /// Title formatting defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub titles: Option<TitlesConfig>,
    /// Page range formatting (expanded, minimal, chicago).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_range_format: Option<PageRangeFormat>,
    /// Bibliography-specific settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<BibliographyConfig>,
    /// Whether to place periods/commas inside quotation marks.
    /// true = American style ("text."), false = British style ("text".)
    /// Defaults to false; en-US locale typically sets this to true.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub punctuation_in_quote: bool,
    /// Delimiter between volume/issue and pages for serial sources.
    /// Processor adds trailing space when rendering.
    /// Examples: Comma (APA ", "), Colon (Chicago ": ").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_pages_delimiter: Option<DelimiterPunctuation>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Page range formatting options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
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

/// Title formatting configuration by title type.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TitlesConfig {
    /// Formatting for component titles (articles, chapters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<TitleRendering>,
    /// Formatting for monograph titles (books).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monograph: Option<TitleRendering>,
    /// Formatting for monograph containers (book containing chapters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_monograph: Option<TitleRendering>,
    /// Formatting for periodical titles (journals, magazines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periodical: Option<TitleRendering>,
    /// Formatting for serial titles (series).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<TitleRendering>,
    /// Default formatting for all titles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<TitleRendering>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Structured link options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct LinksConfig {
    /// Link value to the item's DOI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<bool>,
    /// Link value to the item's URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<bool>,
}

/// Rendering options for titles.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TitleRendering {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl TitleRendering {
    pub fn to_rendering(&self) -> crate::template::Rendering {
        crate::template::Rendering {
            emph: self.emph,
            quote: self.quote,
            strong: self.strong,
            small_caps: self.small_caps,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            ..Default::default()
        }
    }
}

/// Processing mode for citation/bibliography generation.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Processing {
    #[default]
    AuthorDate,
    Numeric,
    Note,
    Custom(ProcessingCustom),
}

/// Custom processing configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ProcessingCustom {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Sort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Group>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguate: Option<Disambiguation>,
}

impl Processing {
    /// Get the effective configuration for this processing mode.
    pub fn config(&self) -> ProcessingCustom {
        match self {
            Processing::AuthorDate => ProcessingCustom {
                sort: Some(Sort {
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
                    ],
                }),
                group: Some(Group {
                    template: vec![SortKey::Author, SortKey::Year],
                }),
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: true,
                    year_suffix: true,
                }),
            },
            Processing::Numeric => ProcessingCustom {
                sort: None,
                group: None,
                disambiguate: None,
            },
            Processing::Note => ProcessingCustom {
                sort: None,
                group: None,
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: false,
                    year_suffix: false,
                }),
            },
            Processing::Custom(custom) => custom.clone(),
        }
    }
}

/// Disambiguation settings.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Disambiguation {
    pub names: bool,
    #[serde(default)]
    pub add_givenname: bool,
    pub year_suffix: bool,
}

impl Default for Disambiguation {
    fn default() -> Self {
        Self {
            names: true,
            add_givenname: false,
            year_suffix: false,
        }
    }
}

/// Date formatting configuration.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DateConfig {
    pub month: MonthFormat,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

impl Default for DateConfig {
    fn default() -> Self {
        Self {
            month: MonthFormat::Long,
            _extra: HashMap::new(),
        }
    }
}

/// Month display format.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MonthFormat {
    #[default]
    Long,
    Short,
    Numeric,
}

/// Contributor formatting configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ContributorConfig {
    /// When to display a contributor's name in sort order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_as_sort: Option<DisplayAsSort>,
    /// String to append after initialized given names (e.g., ". " for "J. Smith").
    /// If None, full given names are used (e.g., "John Smith").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    /// Whether to include a hyphen when initializing names (e.g., "J.-P. Sartre").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with_hyphen: Option<bool>,
    /// Shorten the list of contributors (et al. handling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shorten: Option<ShortenListOptions>,
    /// The delimiter between contributors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Conjunction between last two contributors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<AndOptions>,
    /// When to include delimiter before the last contributor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_last: Option<DelimiterPrecedesLast>,
    /// When to include delimiter before "et al.".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_et_al: Option<DelimiterPrecedesLast>,
    /// When and how to display contributor roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleOptions>,
    /// Handling of non-dropping particles (e.g., "van" in "van Gogh").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demote_non_dropping_particle: Option<DemoteNonDroppingParticle>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Options for demoting non-dropping particles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DemoteNonDroppingParticle {
    Never,
    SortOnly,
    #[default]
    DisplayAndSort,
}

/// When to display names in sort order (family-first).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DisplayAsSort {
    All,
    First,
    #[default]
    None,
}

/// Conjunction options between contributors.
///
/// In CSL 1.0, absence of the `and` attribute means no conjunction.
/// So `None` is the default to match that behavior.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum AndOptions {
    Text,
    Symbol,
    #[default]
    None,
}

/// Role display options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RoleOptions {
    /// Contributor roles for which to omit the role description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omit: Vec<String>,
    /// Global role label form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// Global prefix for role labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Global suffix for role labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Formatting for specific roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<HashMap<String, RoleRendering>>,
}

/// Rendering options for contributor roles.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RoleRendering {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_order: Option<crate::template::NameOrder>,
}

impl RoleRendering {
    pub fn to_rendering(&self) -> crate::template::Rendering {
        crate::template::Rendering {
            emph: self.emph,
            strong: self.strong,
            small_caps: self.small_caps,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            ..Default::default()
        }
    }
}

/// When to use delimiter before last contributor.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterPrecedesLast {
    AfterInvertedName,
    Always,
    Never,
    #[default]
    Contextual,
}

/// Et al. / list shortening options.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ShortenListOptions {
    /// Minimum number of names to trigger shortening.
    pub min: u8,
    /// Number of names to show when shortened.
    pub use_first: u8,
    /// Number of names to show after the ellipsis (et-al-use-last).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_last: Option<u8>,
    /// How to render "and others".
    #[serde(default)]
    pub and_others: AndOtherOptions,
    /// When to use delimiter before last name.
    #[serde(default)]
    pub delimiter_precedes_last: DelimiterPrecedesLast,
}

impl Default for ShortenListOptions {
    fn default() -> Self {
        Self {
            min: 4,
            use_first: 1,
            use_last: None,
            and_others: AndOtherOptions::default(),
            delimiter_precedes_last: DelimiterPrecedesLast::default(),
        }
    }
}

/// How to render "and others" / et al.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AndOtherOptions {
    #[default]
    EtAl,
    Text,
}

/// Localization scope settings.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Localize {
    pub scope: Scope,
}

impl Default for Localize {
    fn default() -> Self {
        Self {
            scope: Scope::Global,
        }
    }
}

/// Localization scope.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    #[default]
    Global,
    PerItem,
}

/// Grouping configuration for bibliography.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Group {
    pub template: Vec<SortKey>,
}

/// Bibliography-specific configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct BibliographyConfig {
    /// String to substitute for repeating authors (e.g., "———").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_author_substitute: Option<String>,
    /// Rule for when to apply the substitute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_author_substitute_rule: Option<SubsequentAuthorSubstituteRule>,
    /// Whether to use a hanging indent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hanging_indent: Option<bool>,
    /// Suffix appended to each bibliography entry (e.g., ".").
    /// Extracted from CSL 1.0 `<layout suffix=".">` attribute.
    /// If None, a trailing period is added by default unless entry ends with DOI/URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_suffix: Option<String>,
    /// Separator between bibliography components (e.g., ". " for Chicago/APA, ", " for Elsevier).
    /// Extracted from CSL 1.0 group delimiter attribute.
    /// Defaults to ". " if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    /// Whether to suppress the trailing period after URLs/DOIs.
    /// Default behavior is to add a period (Chicago, MLA style).
    /// Set to true to suppress the period (APA 7th, Bluebook style).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suppress_period_after_url: bool,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Rules for subsequent author substitution.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SubsequentAuthorSubstituteRule {
    /// Substitute only if ALL authors match.
    #[default]
    CompleteAll,
    /// Substitute each matching name individually.
    CompleteEach,
    /// Substitute each matching name until the first mismatch.
    PartialEach,
    /// Substitute only the first name if it matches.
    PartialFirst,
}

/// Substitution rules for missing author data.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Substitute {
    /// Form to use for contributor roles when substituting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributor_role_form: Option<String>,
    /// Ordered list of fields to try as substitutes.
    pub template: Vec<SubstituteKey>,
}

impl Default for Substitute {
    fn default() -> Self {
        Self {
            contributor_role_form: None,
            template: vec![
                SubstituteKey::Editor,
                SubstituteKey::Title,
                SubstituteKey::Translator,
            ],
        }
    }
}

/// Fields that can be used as author substitutes.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubstituteKey {
    Editor,
    Title,
    Translator,
}

/// Sorting configuration.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Sort {
    /// Shorten name lists for sorting the same as for display.
    #[serde(default)]
    pub shorten_names: bool,
    /// Use same substitutions for sorting as for rendering.
    #[serde(default)]
    pub render_substitutions: bool,
    /// Sort keys in order.
    pub template: Vec<SortSpec>,
}

/// A single sort specification.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SortSpec {
    pub key: SortKey,
    #[serde(default = "default_ascending")]
    pub ascending: bool,
}

fn default_ascending() -> bool {
    true
}

/// Available sort keys.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SortKey {
    #[default]
    Author,
    Year,
    Title,
    /// Sort by citation order (for numeric styles).
    CitationNumber,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(config.disambiguate.unwrap().year_suffix);
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
}
