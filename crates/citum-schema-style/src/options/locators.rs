/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Locator rendering configuration.
//!
//! Defines how citation locators (page numbers, sections, etc.) are displayed,
//! including label forms, range formatting, and compound locator patterns.

use super::{RangeFormat, TextCase};
use crate::template::DelimiterPunctuation;
use citum_schema_data::citation::LocatorType;
use std::collections::HashMap;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a locator label is displayed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelForm {
    /// No label, bare value: "33"
    None,
    /// Short form: "p. 33"
    #[default]
    Short,
    /// Long form: "page 33"
    Long,
    /// Symbol form if available in locale
    Symbol,
}

/// Whether labels appear on every segment, only the first, or none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelRepeat {
    /// Label on every segment
    #[default]
    All,
    /// Label only on the first segment
    First,
    /// No labels
    None,
}

/// A coarse reference genre used as an optional gate on locator patterns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TypeClass {
    /// Legal citations (e.g., "legal-case", "statute")
    Legal,
    /// Classical works with traditional numbering
    Classical,
    /// Standard reference types
    #[default]
    Standard,
}

/// Per-locator-kind configuration overrides.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct LocatorKindConfig {
    /// Override the default label form for this locator kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_form: Option<LabelForm>,
    /// Override the global range format for this locator kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_format: Option<RangeFormat>,
    /// Strip trailing periods from labels (e.g., "p." → "p").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_label_periods: Option<bool>,
    /// Text-case transform applied to this kind's rendered label term.
    /// `AsIs` opts a kind out of a config-level `label_case`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_case: Option<TextCase>,
    /// Overrides the delimiter joining this locator to its preceding
    /// sibling, when the locator is a top-level item in the citation or
    /// integral template. See `docs/specs/LOCATOR_RENDERING.md`
    /// ("Label Case and Attachment") for the scope and precedence rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<DelimiterPunctuation>,
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

impl LocatorKindConfig {
    /// Merge `other`'s `Some` fields into `self`, field by field.
    ///
    /// Used when a `PresetWithOverrides` overlay's `kinds` map shares a key
    /// with the resolved preset's own `kinds` map: the overlay's fields win,
    /// but fields the overlay left unset keep the preset's value (e.g. the
    /// `note` preset's `page.label_form: Some(None)` survives an overlay
    /// that only sets `page.attach`).
    fn merge(&mut self, other: LocatorKindConfig) {
        if other.label_form.is_some() {
            self.label_form = other.label_form;
        }
        if other.range_format.is_some() {
            self.range_format = other.range_format;
        }
        if other.strip_label_periods.is_some() {
            self.strip_label_periods = other.strip_label_periods;
        }
        if other.label_case.is_some() {
            self.label_case = other.label_case;
        }
        if other.attach.is_some() {
            self.attach = other.attach;
        }
    }
}

/// A pattern matching a specific combination of LocatorType values.
///
/// Patterns are tested in declaration order; first match wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct LocatorPattern {
    /// The set of LocatorType values this pattern matches (order-insensitive).
    pub kinds: Vec<LocatorType>,
    /// Optional gate on reference type class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_class: Option<TypeClass>,
    /// Rendering order of segments when pattern matches.
    pub order: Vec<LocatorType>,
    /// Delimiter between segments (default: ", ").
    #[serde(default = "default_delimiter")]
    pub delimiter: String,
    /// Whether labels appear on every segment, only the first, or none.
    #[serde(default)]
    pub label_repeat: LabelRepeat,
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

/// Top-level locator rendering configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct LocatorConfig {
    /// Default label form for all locator kinds (default: Short).
    #[serde(default = "default_label_form")]
    pub default_label_form: LabelForm,
    /// Range format for all locator kinds. `None` inherits the style-wide
    /// `options.range-format` default (see
    /// `docs/specs/RANGE_COLLAPSE_MODEL.md` Decision 2); `Some` overrides it
    /// for every kind unless a `kinds.<kind>.range-format` entry wins first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_format: Option<RangeFormat>,
    /// Strip trailing periods from labels globally (e.g., "p." → "p").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_label_periods: Option<bool>,
    /// Default label-case transform for all kinds unless overridden per kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_case: Option<TextCase>,
    /// Default attachment delimiter for all kinds unless overridden per kind.
    /// See `LocatorKindConfig::attach` for scope and precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<DelimiterPunctuation>,
    /// Per-kind configuration overrides.
    #[serde(default)]
    pub kinds: HashMap<LocatorType, LocatorKindConfig>,
    /// Patterns for compound locators and type-specific rendering.
    #[serde(default)]
    pub patterns: Vec<LocatorPattern>,
    /// Fallback delimiter for unmatched compound locators (default: ", ").
    #[serde(default = "default_delimiter")]
    pub fallback_delimiter: String,
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

impl Default for LocatorConfig {
    fn default() -> Self {
        Self {
            default_label_form: LabelForm::Short,
            range_format: None,
            strip_label_periods: None,
            label_case: None,
            attach: None,
            kinds: HashMap::new(),
            patterns: Vec::new(),
            fallback_delimiter: ", ".to_string(),
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }
}

/// Named presets for common locator configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LocatorPreset {
    /// Note style: bare page numbers, short labels for other kinds.
    Note,
    /// Author-date / numbered: short labels for all kinds.
    AuthorDate,
    /// Numeric journal convention: same as `author-date`, but strips periods from locator
    /// labels (e.g. "p." becomes "p"). Common across Vancouver-family medical/science journals.
    Numeric,
}

impl LocatorPreset {
    /// All current variants. See `ContributorPreset::ALL` in `citum-schema-style::presets` for
    /// why this exists.
    pub const ALL: &'static [LocatorPreset] = &[
        LocatorPreset::Note,
        LocatorPreset::AuthorDate,
        LocatorPreset::Numeric,
    ];

    /// Resolve a preset to an explicit `LocatorConfig`.
    #[must_use]
    pub fn config(self) -> LocatorConfig {
        match self {
            LocatorPreset::Note => LocatorConfig {
                default_label_form: LabelForm::Short,
                range_format: None,
                strip_label_periods: None,
                label_case: None,
                attach: None,
                kinds: {
                    let mut m = HashMap::new();
                    // Page locators have no label in note style
                    m.insert(
                        LocatorType::Page,
                        LocatorKindConfig {
                            label_form: Some(LabelForm::None),
                            range_format: None,
                            strip_label_periods: None,
                            label_case: None,
                            attach: None,
                            unknown_fields: std::collections::BTreeMap::new(),
                        },
                    );
                    m
                },
                patterns: Vec::new(),
                fallback_delimiter: ", ".to_string(),
                unknown_fields: std::collections::BTreeMap::new(),
            },
            LocatorPreset::AuthorDate => LocatorConfig {
                default_label_form: LabelForm::Short,
                range_format: None,
                strip_label_periods: None,
                label_case: None,
                attach: None,
                kinds: HashMap::new(),
                patterns: Vec::new(),
                fallback_delimiter: ", ".to_string(),
                unknown_fields: std::collections::BTreeMap::new(),
            },
            LocatorPreset::Numeric => LocatorConfig {
                default_label_form: LabelForm::Short,
                range_format: None,
                strip_label_periods: Some(true),
                label_case: None,
                attach: None,
                kinds: HashMap::new(),
                patterns: Vec::new(),
                fallback_delimiter: ", ".to_string(),
                unknown_fields: std::collections::BTreeMap::new(),
            },
        }
    }
}

/// All-`Option` overlay applied on top of a resolved preset.
///
/// `LocatorConfig` itself cannot serve as the overlay: its
/// `default_label_form` and `fallback_delimiter` fields are non-`Option`
/// with serde defaults, so a flattened `LocatorConfig` overlay could not
/// distinguish "field not set, inherit the preset" from "field explicitly
/// set to the schema default." Every field here mirrors a field on
/// `LocatorConfig`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default)]
pub struct LocatorOverrides {
    /// Overrides the preset's default label form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_label_form: Option<LabelForm>,
    /// Overrides the preset's range format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_format: Option<RangeFormat>,
    /// Overrides the preset's `strip_label_periods`. `Some(false)` is an
    /// explicit clear, distinct from an omitted field (which inherits).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_label_periods: Option<bool>,
    /// Overrides the preset's `label_case`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_case: Option<TextCase>,
    /// Overrides the preset's `attach`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<DelimiterPunctuation>,
    /// Overrides the preset's fallback delimiter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_delimiter: Option<String>,
    /// Merged into the preset's `kinds` map per key, not replaced wholesale.
    #[serde(default)]
    pub kinds: HashMap<LocatorType, LocatorKindConfig>,
    /// Replaces the preset's `patterns` wholesale when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<Vec<LocatorPattern>>,
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

impl LocatorOverrides {
    /// Overlay `self` onto a resolved preset `LocatorConfig`, field by field.
    ///
    /// `kinds` merges per `LocatorType` key into the base's `kinds` map
    /// rather than replacing it wholesale, so a preset's own per-kind
    /// entries (e.g. `note`'s bare-page `LocatorKindConfig`) survive unless
    /// the same key is present in the overlay.
    #[must_use]
    pub fn apply(self, mut base: LocatorConfig) -> LocatorConfig {
        if let Some(v) = self.default_label_form {
            base.default_label_form = v;
        }
        if self.range_format.is_some() {
            base.range_format = self.range_format;
        }
        if self.strip_label_periods.is_some() {
            base.strip_label_periods = self.strip_label_periods;
        }
        if self.label_case.is_some() {
            base.label_case = self.label_case;
        }
        if self.attach.is_some() {
            base.attach = self.attach;
        }
        if let Some(v) = self.fallback_delimiter {
            base.fallback_delimiter = v;
        }
        if let Some(v) = self.patterns {
            base.patterns = v;
        }
        for (kind, overlay_kind_cfg) in self.kinds {
            match base.kinds.entry(kind) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    e.get_mut().merge(overlay_kind_cfg);
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(overlay_kind_cfg);
                }
            }
        }
        base
    }
}

/// Preset-or-explicit entry — same pattern as DateConfigEntry.
///
/// `#[serde(untagged)]`: variant order is load-bearing. `Preset` is tried
/// first (a bare string). `PresetWithOverrides` is tried next — its
/// `preset` field is required, so it only matches a mapping carrying that
/// key, and falls through otherwise. `Explicit` **must come last**:
/// `LocatorConfig` carries a `#[serde(flatten)] unknown_fields` catch-all
/// with no `deny_unknown_fields`, so `Explicit` would happily accept (and
/// silently discard into `unknown_fields`) a stray `preset` key. Trying
/// `Explicit` before `PresetWithOverrides` would make
/// `{preset: note, kinds: {...}}` resolve as a bare `Explicit` with
/// `preset` discarded and no preset behavior applied at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum LocatorConfigEntry {
    /// A preset name.
    Preset(LocatorPreset),
    /// A preset resolved to a `LocatorConfig`, then overlaid with `overrides`.
    PresetWithOverrides {
        /// The base preset to resolve before applying `overrides`.
        preset: LocatorPreset,
        /// Fields to overlay onto the resolved preset.
        #[serde(flatten)]
        overrides: LocatorOverrides,
    },
    /// Explicit configuration.
    Explicit(LocatorConfig),
}

impl LocatorConfigEntry {
    /// Resolve a LocatorConfigEntry to an explicit LocatorConfig.
    #[must_use]
    pub fn resolve(self) -> LocatorConfig {
        match self {
            LocatorConfigEntry::Preset(preset) => preset.config(),
            LocatorConfigEntry::PresetWithOverrides { preset, overrides } => {
                overrides.apply(preset.config())
            }
            LocatorConfigEntry::Explicit(config) => config,
        }
    }
}

/// Default label form.
fn default_label_form() -> LabelForm {
    LabelForm::Short
}

/// Default delimiter string.
fn default_delimiter() -> String {
    ", ".to_string()
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
    fn test_locator_preset_note() {
        let config = LocatorPreset::Note.config();
        assert_eq!(config.default_label_form, LabelForm::Short);
        // `None` inherits the style-wide default (Decision 2), not a
        // preset-hardcoded `Expanded`.
        assert_eq!(config.range_format, None);
    }

    #[test]
    fn test_locator_preset_author_date() {
        let config = LocatorPreset::AuthorDate.config();
        assert_eq!(config.default_label_form, LabelForm::Short);
        assert_eq!(config.range_format, None);
    }

    #[test]
    fn test_locator_preset_numeric() {
        let config = LocatorPreset::Numeric.config();
        // Numeric is author-date plus strip-label-periods; everything else matches author-date.
        let author_date = LocatorPreset::AuthorDate.config();
        assert_eq!(config.strip_label_periods, Some(true));
        assert_eq!(config.default_label_form, author_date.default_label_form);
        assert_eq!(config.range_format, author_date.range_format);
        assert_eq!(config.fallback_delimiter, author_date.fallback_delimiter);
    }

    #[test]
    fn test_locator_preset_all_covers_every_variant() {
        // Every branch of LocatorPreset::config()'s match arm must have a corresponding entry in
        // ALL, or the analyzer's reverse-match set silently omits a named preset.
        assert_eq!(LocatorPreset::ALL.len(), 3);
        for preset in LocatorPreset::ALL {
            let _ = preset.config();
        }
    }

    #[test]
    fn test_locator_config_entry_preset() {
        let entry = LocatorConfigEntry::Preset(LocatorPreset::Note);
        let config = entry.resolve();
        assert_eq!(config.default_label_form, LabelForm::Short);
    }

    #[test]
    fn test_numeric_preset_name_resolves_through_config_entry() {
        let entry: LocatorConfigEntry = serde_yaml::from_str("numeric").unwrap();
        assert_eq!(entry.resolve(), LocatorPreset::Numeric.config());
    }

    #[test]
    fn test_locator_config_entry_explicit() {
        let entry = LocatorConfigEntry::Explicit(LocatorConfig {
            default_label_form: LabelForm::Long,
            ..Default::default()
        });
        let config = entry.resolve();
        assert_eq!(config.default_label_form, LabelForm::Long);
    }

    #[test]
    fn test_locator_config_default() {
        let config = LocatorConfig::default();
        assert_eq!(config.default_label_form, LabelForm::Short);
        assert_eq!(config.fallback_delimiter, ", ");
        assert_eq!(config.range_format, None);
    }

    #[rstest]
    #[case::page("page", LocatorType::Page)]
    #[case::line("line", LocatorType::Line)]
    fn given_a_preset_with_a_per_kind_attach_override_when_parsed_then_resolves_as_preset_with_overrides(
        #[case] kind_key: &str,
        #[case] overridden_kind: LocatorType,
    ) {
        let yaml = format!("preset: note\nkinds:\n  {kind_key}:\n    attach: \" \"\n");
        let entry: LocatorConfigEntry = serde_yaml::from_str(&yaml).unwrap();
        assert!(
            matches!(entry, LocatorConfigEntry::PresetWithOverrides { .. }),
            "an untagged {{preset, kinds}} mapping must resolve as PresetWithOverrides, \
             not fall through to Explicit and silently discard `preset`"
        );

        let config = entry.resolve();
        // The `note` preset's own bare-page behavior must survive the
        // overlay: the overlay only adds an `attach` entry for one kind,
        // it must not replace the preset's `kinds` map wholesale.
        assert_eq!(
            config
                .kinds
                .get(&LocatorType::Page)
                .and_then(|k| k.label_form),
            Some(LabelForm::None)
        );
        assert_eq!(
            config
                .kinds
                .get(&overridden_kind)
                .and_then(|k| k.attach.clone()),
            Some(DelimiterPunctuation::Custom(" ".to_string()))
        );
    }

    #[test]
    fn given_a_plain_explicit_locator_config_when_parsed_then_resolves_as_explicit_not_preset_with_overrides()
     {
        let yaml = "default-label-form: long\nlabel-case: capitalize-first\n";
        let entry: LocatorConfigEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(entry, LocatorConfigEntry::Explicit(_)));

        let config = entry.resolve();
        assert_eq!(config.default_label_form, LabelForm::Long);
        assert_eq!(config.label_case, Some(TextCase::CapitalizeFirst));
    }

    #[test]
    fn given_a_preset_with_overrides_strip_label_periods_false_when_resolved_then_it_clears_the_preset_value()
     {
        // `numeric` sets strip_label_periods: Some(true); an explicit
        // `Some(false)` override must clear it, distinct from omitting the
        // field (which would inherit the preset's `true`).
        let entry = LocatorConfigEntry::PresetWithOverrides {
            preset: LocatorPreset::Numeric,
            overrides: LocatorOverrides {
                strip_label_periods: Some(false),
                ..Default::default()
            },
        };
        let config = entry.resolve();
        assert_eq!(config.strip_label_periods, Some(false));
    }

    #[test]
    fn given_a_kind_level_label_case_as_is_when_resolved_then_it_is_distinct_from_no_override() {
        let with_override = LocatorKindConfig {
            label_case: Some(TextCase::AsIs),
            ..Default::default()
        };
        let without_override = LocatorKindConfig::default();
        assert_ne!(with_override.label_case, without_override.label_case);
    }
}
