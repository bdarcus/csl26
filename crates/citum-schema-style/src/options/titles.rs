/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{ReferenceTypeName, TitleCategory};

/// Text-case transform applied to title-like fields.
///
/// Styles select which transform applies to which field category.
/// The engine provides the generic primitives; styles own the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TextCase {
    /// English headline-style title case (capitalize principal words).
    Title,
    /// Generic sentence case (capitalize first word only).
    Sentence,
    /// APA-style sentence case: capitalize first word of main title
    /// and first word after each subtitle boundary.
    SentenceApa,
    /// NLM-style sentence case: capitalize first word of main title only;
    /// subtitles preserve only explicit/protected capitals.
    SentenceNlm,
    /// Capitalize the first letter of the value.
    CapitalizeFirst,
    /// Transform the entire value to lowercase.
    Lowercase,
    /// Transform the entire value to uppercase.
    Uppercase,
    /// No transformation; render the value exactly as stored.
    AsIs,
}

/// Title config: either a preset name or explicit configuration.
///
/// Allows styles to write `titles: apa` as shorthand, or provide
/// full explicit configuration with field-level overrides.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum TitlesConfigEntry {
    /// A named preset (e.g., "apa", "chicago", "humanities", "scientific").
    Preset(crate::presets::TitlePreset),
    /// Explicit title configuration.
    Explicit(Box<TitlesConfig>),
}

impl Default for TitlesConfigEntry {
    fn default() -> Self {
        TitlesConfigEntry::Explicit(Box::default())
    }
}

impl TitlesConfigEntry {
    /// Resolve this entry to a concrete `TitlesConfig`.
    pub fn resolve(&self) -> TitlesConfig {
        match self {
            TitlesConfigEntry::Preset(preset) => preset.config(),
            TitlesConfigEntry::Explicit(config) => *config.clone(),
        }
    }
}

/// Title formatting configuration by title type.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct TitlesConfig {
    /// Mapping of canonical reference types to title categories.
    ///
    /// Unknown reference types are retained for forward compatibility and
    /// reported by style validation. Category values are a closed vocabulary.
    /// Example: { "thesis": "monograph", "article-journal": "periodical" }
    /// `None` means "not configured at this level" and will not override an
    /// inherited value; explicit `type-mapping: ~` clears an inherited
    /// mapping entirely (see `docs/specs/STYLE_INHERITANCE.md`).
    #[serde(
        default,
        deserialize_with = "deserialize_type_mapping",
        skip_serializing_if = "Option::is_none"
    )]
    pub type_mapping: Option<HashMap<ReferenceTypeName, TitleCategory>>,
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

impl TitlesConfig {
    /// Return the authored category for a reference type, using canonical
    /// underscore-to-hyphen normalization for lookup.
    #[must_use]
    pub fn mapped_category(&self, reference_type: &str) -> Option<TitleCategory> {
        let key = ReferenceTypeName::from(reference_type);
        self.type_mapping.as_ref()?.get(key.as_str()).copied()
    }
}

struct TypeMapping(HashMap<ReferenceTypeName, TitleCategory>);

impl<'de> Deserialize<'de> for TypeMapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TypeMappingVisitor;

        impl<'de> serde::de::Visitor<'de> for TypeMappingVisitor {
            type Value = TypeMapping;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a reference-type to title-category mapping")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut result = HashMap::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((raw_key, category)) = map.next_entry::<String, TitleCategory>()? {
                    let key = ReferenceTypeName::new(raw_key);
                    if result.insert(key.clone(), category).is_some() {
                        return Err(A::Error::custom(format!(
                            "duplicate title type-mapping key `{key}` after normalization"
                        )));
                    }
                }
                Ok(TypeMapping(result))
            }
        }

        deserializer.deserialize_map(TypeMappingVisitor)
    }
}

fn deserialize_type_mapping<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<ReferenceTypeName, TitleCategory>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<TypeMapping>::deserialize(deserializer).map(|mapping| mapping.map(|value| value.0))
}

/// Rendering options for titles.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct TitleRendering {
    /// Text-case transform to apply to this title category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_case: Option<TextCase>,
    /// Delimiter between the main title and the first subtitle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_delimiter: Option<String>,
    /// Delimiter between subtitle parts when a title has multiple subtitles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle_delimiter: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_overrides: Option<HashMap<String, TitleRendering>>,
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

impl TitleRendering {
    /// Convert title rendering fields shared by generic components into a
    /// template rendering configuration.
    pub fn to_rendering(&self) -> crate::template::Rendering {
        crate::template::Rendering {
            text_case: self.text_case,
            emph: self.emph,
            quote: self.quote,
            strong: self.strong,
            small_caps: self.small_caps,
            prefix: self.prefix.clone().map(Into::into),
            suffix: self.suffix.clone().map(Into::into),
            ..Default::default()
        }
    }

    /// Merge another title rendering configuration into this one.
    ///
    /// The other rendering takes precedence, overwriting any fields that are present.
    pub fn merge(&mut self, other: &TitleRendering) {
        crate::merge_options!(
            self,
            other,
            text_case,
            primary_delimiter,
            subtitle_delimiter,
            emph,
            quote,
            strong,
            small_caps,
            prefix,
            suffix,
            locale_overrides,
        );
    }

    /// Return the most specific language override for a language tag.
    pub fn locale_override(&self, language: Option<&str>) -> Option<&TitleRendering> {
        let overrides = self.locale_overrides.as_ref()?;
        let language = language?;
        overrides.get(language).or_else(|| {
            language
                .split('-')
                .next()
                .and_then(|tag| overrides.get(tag))
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use crate::presets::TitlePreset;

    #[test]
    fn test_emphasis_all_preset_name_resolves_through_config_entry() {
        let entry: TitlesConfigEntry = serde_yaml::from_str("emphasis-all").unwrap();
        assert_eq!(entry.resolve(), TitlePreset::EmphasisAll.config());
    }

    #[test]
    fn test_title_case_preset_name_resolves_through_config_entry() {
        let entry: TitlesConfigEntry = serde_yaml::from_str("title-case").unwrap();
        assert_eq!(entry.resolve(), TitlePreset::TitleCase.config());
    }

    #[test]
    fn type_mapping_rejects_invalid_category() {
        let error = serde_yaml::from_str::<TitlesConfig>(
            "type-mapping:\n  article-journal: not-a-title-category\n",
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unknown variant `not-a-title-category`")
        );
    }

    #[test]
    fn type_mapping_normalizes_underscore_keys_for_lookup_and_serialization() {
        let config: TitlesConfig =
            serde_yaml::from_str("type-mapping:\n  personal_communication: component\n").unwrap();

        assert_eq!(
            config.mapped_category("personal-communication"),
            Some(TitleCategory::Component)
        );
        assert_eq!(
            serde_yaml::to_string(&config).unwrap(),
            "type-mapping:\n  personal-communication: component\n"
        );
    }

    #[test]
    fn type_mapping_rejects_duplicate_keys_after_normalization() {
        let error = serde_yaml::from_str::<TitlesConfig>(
            "type-mapping:\n  article_journal: component\n  article-journal: periodical\n",
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("duplicate title type-mapping key `article-journal` after normalization")
        );
    }

    #[test]
    fn type_mapping_accepts_collection_as_container_monograph_alias() {
        let config: TitlesConfig =
            serde_yaml::from_str("type-mapping:\n  chapter: collection\n").unwrap();

        assert_eq!(
            config.mapped_category("chapter"),
            Some(TitleCategory::ContainerMonograph)
        );
        assert_eq!(
            serde_yaml::to_string(&config).unwrap(),
            "type-mapping:\n  chapter: container-monograph\n"
        );
    }

    #[cfg(feature = "schema")]
    #[test]
    fn title_category_schema_includes_collection_compatibility_alias() {
        let schema = serde_json::to_value(schemars::schema_for!(TitleCategory)).unwrap();

        assert_eq!(
            schema.get("enum"),
            Some(&serde_json::json!([
                "component",
                "monograph",
                "periodical",
                "serial",
                "container-monograph",
                "collection",
                "default"
            ]))
        );
    }
}
