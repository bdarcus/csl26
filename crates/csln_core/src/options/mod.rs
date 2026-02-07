/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Style configuration options.

pub mod bibliography;
pub mod contributors;
pub mod dates;
pub mod localization;
pub mod processing;
pub mod substitute;

pub use bibliography::{BibliographyConfig, SubsequentAuthorSubstituteRule};
pub use contributors::{
    AndOptions, AndOtherOptions, ContributorConfig, DelimiterPrecedesLast,
    DemoteNonDroppingParticle, DisplayAsSort, EditorLabelFormat, RoleOptions, RoleRendering,
    ShortenListOptions,
};
pub use dates::DateConfig;
pub use localization::{Localize, MonthFormat, Scope};
pub use processing::{
    Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
};
pub use substitute::{Substitute, SubstituteConfig, SubstituteKey};

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
    pub substitute: Option<SubstituteConfig>,
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
    pub titles: Option<crate::options::titles::TitlesConfig>,
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

pub mod titles;

pub use titles::{TitleRendering, TitlesConfig};

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
}
