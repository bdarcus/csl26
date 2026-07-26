/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::options::localization::MonthFormat;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Time display format.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TimeFormat {
    /// 12-hour clock with AM/PM (e.g., "11:30 PM")
    Hour12,
    /// 24-hour clock (e.g., "23:30")
    Hour24,
}

/// Era label profile for date rendering.
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EraLabels {
    /// Preserve current behavior: negative years use locale `before-era`, positive years unlabeled.
    #[default]
    Default,
    /// Negative years use locale `bc`, positive years use locale `ad`.
    BcAd,
    /// Negative years use locale `bce`, positive years use locale `ce`.
    BceCe,
}

/// Rendering policy for negative EDTF years with unspecified digits.
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NegativeUnspecifiedYears {
    /// Render as explicit historical ranges (e.g., `-009u` → `100–91 BC`).
    #[default]
    Range,
    /// Reserved for future prose-oriented output; falls back to `range` if selected.
    Fuzzy,
}

/// Formatting policy for closed EDTF year ranges.
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateRangeFormat {
    /// Render both endpoints in full (for example, `2021–2026`).
    #[default]
    Expanded,
    /// Apply Chicago inclusive-number condensation (for example, `2021–26`).
    Chicago,
}

/// Term form for the "no date" fallback when `issued` is empty.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NoDateForm {
    /// Render the locale's short term (e.g. "n.d.").
    #[default]
    Short,
    /// Render the locale's long term (e.g. "no date").
    Long,
}

/// Date config: either a preset name or explicit configuration.
///
/// Allows styles to write `dates: long` as shorthand, or provide
/// full explicit configuration with field-level overrides.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[allow(
    clippy::large_enum_variant,
    reason = "Boxing the explicit configuration would break the public configuration API."
)]
#[serde(untagged)]
pub enum DateConfigEntry {
    /// A named preset (e.g., "long", "short", "numeric", "iso").
    Preset(crate::presets::DatePreset),
    /// Explicit date configuration.
    Explicit(DateConfig),
}

impl Default for DateConfigEntry {
    fn default() -> Self {
        DateConfigEntry::Explicit(DateConfig::default())
    }
}

impl DateConfigEntry {
    /// Resolve this entry to a concrete `DateConfig`.
    pub fn resolve(&self) -> DateConfig {
        match self {
            DateConfigEntry::Preset(preset) => preset.config(),
            DateConfigEntry::Explicit(config) => config.clone(),
        }
    }
}

/// Date formatting configuration.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct DateConfig {
    pub month: MonthFormat,
    /// Marker for uncertain dates (e.g., "?" or "uncertain"). None suppresses display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncertainty_marker: Option<String>,
    /// Marker for approximate dates (e.g., "ca. " or "~"). None suppresses display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximation_marker: Option<String>,
    /// Optional closing marker paired with `approximation_marker`, for
    /// bracket-style approximate-date notation (e.g. GB/T 7714's `[1936]`
    /// estimated year, where the marker is `[` and this is `]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximation_marker_suffix: Option<String>,
    /// Delimiter for date ranges (default: en-dash "–").
    #[serde(default = "default_range_delimiter")]
    pub range_delimiter: String,
    /// Formatting policy for closed year ranges (default: expanded).
    #[serde(default)]
    pub range_format: DateRangeFormat,
    /// Marker for open-ended ranges (e.g., "–present"). None uses locale default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_range_marker: Option<String>,
    /// Locale term form used for the "no date" fallback when a template's
    /// `issued` date is empty: `short` renders "n.d.", `long` renders
    /// "no date". Defaults to short.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_date_form: Option<NoDateForm>,
    /// Delimiter inserted between the no-date term and a year-suffix disambiguator.
    #[serde(default = "default_no_date_year_suffix_delimiter")]
    pub no_date_year_suffix_delimiter: String,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
    /// Time display format. None suppresses time rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<TimeFormat>,
    /// Whether to include seconds in time display (default: false).
    #[serde(default)]
    pub show_seconds: bool,
    /// Whether to include timezone in time display (default: false).
    #[serde(default)]
    pub show_timezone: bool,
    /// Era label profile controlling which era suffixes are shown.
    #[serde(default)]
    pub era_labels: EraLabels,
    /// How negative EDTF years with unspecified digits are rendered.
    #[serde(default)]
    pub negative_unspecified_years: NegativeUnspecifiedYears,
    /// Wrap applied around a date's opaque `note` (e.g. a source-calendar
    /// annotation), appended after the complete formatted date. `None`
    /// (the default) hides the note entirely, even when the input has one.
    /// See `docs/specs/CALENDAR_DATE_ANNOTATIONS.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_wrap: Option<crate::template::WrapConfig>,
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

fn default_range_delimiter() -> String {
    "–".to_string() // U+2013 en-dash
}

fn default_no_date_year_suffix_delimiter() -> String {
    "-".to_string()
}

impl Default for DateConfig {
    fn default() -> Self {
        Self {
            month: MonthFormat::Long,
            uncertainty_marker: Some("?".to_string()),
            approximation_marker: Some("ca. ".to_string()),
            approximation_marker_suffix: None,
            range_delimiter: default_range_delimiter(),
            range_format: DateRangeFormat::default(),
            open_range_marker: None,
            no_date_form: None,
            no_date_year_suffix_delimiter: default_no_date_year_suffix_delimiter(),
            custom: None,
            time_format: None,
            show_seconds: false,
            show_timezone: false,
            era_labels: EraLabels::default(),
            negative_unspecified_years: NegativeUnspecifiedYears::default(),
            note_wrap: None,
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Panicking is acceptable in tests.")]
mod tests {
    use super::*;

    #[test]
    fn captures_unknown_fields_for_forward_compat() {
        let yaml = r#"
month: long
future-key: true
"#;
        let cfg: DateConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.unknown_fields.contains_key("future-key"));
        assert_eq!(cfg.month, MonthFormat::Long);
    }

    #[test]
    fn defaults_no_date_year_suffix_delimiter_to_hyphen() {
        assert_eq!(DateConfig::default().no_date_year_suffix_delimiter, "-");
    }

    #[test]
    fn defaults_range_format_to_expanded() {
        assert!(matches!(
            DateConfig::default().range_format,
            DateRangeFormat::Expanded
        ));
    }

    #[test]
    fn parses_chicago_range_format() {
        let yaml = r#"
month: long
range-format: chicago
"#;
        let cfg: DateConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(matches!(cfg.range_format, DateRangeFormat::Chicago));
    }

    #[test]
    fn parses_no_date_year_suffix_delimiter() {
        let yaml = r#"
month: long
no-date-year-suffix-delimiter: ""
"#;
        let cfg: DateConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(cfg.no_date_year_suffix_delimiter, "");
    }
}
