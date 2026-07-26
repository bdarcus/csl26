/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de};
use std::collections::HashMap;

/// Multilingual rendering options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualConfig {
    /// Preferred rendering mode for titles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_mode: Option<MultilingualMode>,
    /// Preferred rendering mode for names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_mode: Option<MultilingualMode>,
    /// Preferred script for transliterations (e.g., "Latn").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_script: Option<String>,
    /// Ordered priority list of BCP 47 transliteration tags (e.g. `["ja-Latn-hepburn", "ja-Latn"]`).
    /// Takes precedence over `preferred_script` when resolving transliterations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_transliteration: Option<Vec<String>>,
    /// Script-specific behavior configuration.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub scripts: HashMap<String, ScriptConfig>,
    /// Script class to realize semantic wrap punctuation (`wrap: parentheses`/`brackets`)
    /// as, for items with no usable script evidence. Unset defaults to `latin`,
    /// matching today's unconditional half-width output. See
    /// `docs/specs/PUNCTUATION_REALIZATION.md` §5.
    #[serde(
        default,
        rename = "realization-default",
        skip_serializing_if = "RealizationDefault::is_latin"
    )]
    pub realization_default: RealizationDefault,
    /// Named punctuation realization preset. When absent, the legacy
    /// `realization-default` behavior remains in effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation_width: Option<PunctuationWidth>,
    /// Whether locale-sensitive term/message/date-pattern lookups (roles,
    /// locators, terms, date names and patterns) resolve against the style
    /// locale or each item's own effective language. Unset defaults to
    /// `style`, matching today's behavior byte for byte. Typography
    /// (`grammar-options`) always stays with the style locale regardless of
    /// this setting. See `docs/specs/PER_ITEM_TERM_LOCALE.md`.
    #[serde(
        default,
        rename = "term-locale",
        skip_serializing_if = "TermLocale::is_style"
    )]
    pub term_locale: TermLocale,
}

impl MultilingualConfig {
    /// Merge another `MultilingualConfig` into this one, with `other` taking
    /// precedence field by field.
    ///
    /// Without this, `Config::merge` replaces the whole block, so a style that
    /// extends a parent and sets any single multilingual key silently discards
    /// every inherited one — e.g. adding a `scripts` entry to a style extending
    /// `gb-t-7714-2025-numeric` dropped the inherited `punctuation-width: mixed`
    /// and reverted Chinese punctuation to half-width (bean `csl26-p7kj`).
    ///
    /// `scripts` merges per key, so an overlay redefines only the scripts it
    /// names. `realization_default` and `term_locale` are not `Option`, and
    /// their defaults are indistinguishable from "unset" — both are
    /// `skip_serializing_if`-elided at their default, so this treats a default
    /// value as "not specified" and leaves the inherited value in place.
    pub fn merge(&mut self, other: &MultilingualConfig) {
        if other.title_mode.is_some() {
            self.title_mode = other.title_mode.clone();
        }
        if other.name_mode.is_some() {
            self.name_mode = other.name_mode.clone();
        }
        if other.preferred_script.is_some() {
            self.preferred_script = other.preferred_script.clone();
        }
        if other.preferred_transliteration.is_some() {
            self.preferred_transliteration = other.preferred_transliteration.clone();
        }
        if other.punctuation_width.is_some() {
            self.punctuation_width = other.punctuation_width;
        }
        for (script, config) in &other.scripts {
            self.scripts.insert(script.clone(), config.clone());
        }
        if !other.realization_default.is_latin() {
            self.realization_default = other.realization_default;
        }
        if !other.term_locale.is_style() {
            self.term_locale = other.term_locale;
        }
    }
}

/// Which locale a style's engine-supplied terms/messages/date patterns
/// resolve against: the style's own locale, or each rendered item's
/// effective language (the biblatex `autolang` analogue). See
/// `docs/specs/PER_ITEM_TERM_LOCALE.md`.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TermLocale {
    /// Terms always resolve against the style locale (today's behavior).
    #[default]
    Style,
    /// Terms resolve against each item's effective language, falling back
    /// to the style locale when no matching locale is loaded.
    Item,
}

impl TermLocale {
    /// Returns `true` for the default variant (`Style`), used for skip-serializing.
    pub fn is_style(&self) -> bool {
        matches!(self, TermLocale::Style)
    }
}

/// Rendering modes for multilingual content.
#[derive(Debug, PartialEq, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MultilingualMode {
    /// Use original script.
    Primary,
    /// Use transliteration.
    Transliterated,
    /// Use translation matching style locale.
    Translated,
    /// Combine transliteration and translation: `romanized [translated]`.
    /// Equivalent to `Pattern([transliterated, {translated, brackets}])`.
    Combined,
    /// Ordered sequence of views joined by spaces.
    ///
    /// Use this when a style requires more than two views — e.g. Chicago's
    /// `romanized original-script [translated]` or MLA's `original-script [translated]`.
    /// Each segment specifies a view and an optional bracket wrap.
    /// Segments whose resolved text is empty or identical to the previous
    /// segment are silently skipped (dedup).
    ///
    /// YAML form:
    /// ```yaml
    /// title-mode:
    ///   pattern:
    ///     - view: transliterated
    ///     - view: original-script
    ///     - view: translated
    ///       wrap: brackets
    /// ```
    Pattern(Vec<MultilingualSegment>),
}

/// A single view segment in a `MultilingualMode::Pattern`.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualSegment {
    /// Which textual view to render for this segment.
    pub view: MultilingualView,
    /// Optional wrapping applied around the resolved text.
    #[serde(default, skip_serializing_if = "SegmentWrap::is_none")]
    pub wrap: SegmentWrap,
}

/// Which textual representation of a multilingual field to use in a pattern segment.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MultilingualView {
    /// The original-script text as stored in the data.
    OriginalScript,
    /// The transliterated (romanized) form.
    Transliterated,
    /// The translation matching the style locale.
    Translated,
}

/// Bracket wrapping applied to a pattern segment.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SegmentWrap {
    /// No wrapping.
    #[default]
    None,
    /// Wrap in square brackets: `[text]`.
    Brackets,
    /// Wrap in parentheses: `(text)`.
    Parentheses,
}

impl SegmentWrap {
    /// Returns `true` when the variant is `None` (used for skip-serializing).
    pub fn is_none(&self) -> bool {
        matches!(self, SegmentWrap::None)
    }

    /// Apply the wrap to a string slice, returning the wrapped form.
    pub fn apply(&self, text: &str) -> String {
        match self {
            SegmentWrap::None => text.to_string(),
            SegmentWrap::Brackets => format!("[{text}]"),
            SegmentWrap::Parentheses => format!("({text})"),
        }
    }
}

/// Configuration for specific scripts.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct ScriptConfig {
    /// Whether to use native ordering for this script (e.g., FamilyGiven for CJK).
    #[serde(default)]
    pub use_native_ordering: bool,
    /// Custom delimiter between name parts for this script.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Custom delimiter between family and given name when this script is inverted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    /// Punctuation convention to render for items whose effective language
    /// resolves to this script (e.g. remap CJK full-width delimiters to Latin
    /// half-width for Latin-script items in a bilingual style).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation: Option<PunctuationStyle>,
    /// Style-owned glyph overrides for semantic punctuation marks realized for
    /// this script class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realization: Option<PunctuationRealization>,
}

/// Per-script glyph overrides for semantic punctuation marks.
///
/// Scalar marks use complete separator strings, including spacing. Paired
/// marks use `[open, close]`.
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PunctuationRealization {
    /// Override for the semantic comma mark.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comma: Option<String>,
    /// Override for the semantic colon mark.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colon: Option<String>,
    /// Override for the semantic semicolon mark.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semicolon: Option<String>,
    /// Override for the semantic period mark.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    /// Override for the semantic parentheses pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parentheses: Option<[String; 2]>,
    /// Override for the semantic brackets pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brackets: Option<[String; 2]>,
}

/// Punctuation convention applied to a script's rendered output.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum PunctuationStyle {
    /// Half-width Latin delimiters (`: , ( )`), with a trailing space after `:` and `,`.
    Latin,
    /// Full-width CJK delimiters (`： ， （ ）`), the unmodified default.
    FullWidth,
}

/// Script class an item with no usable script evidence realizes semantic wrap
/// punctuation as. See `options.multilingual.realization-default` and
/// `docs/specs/PUNCTUATION_REALIZATION.md` §5.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RealizationDefault {
    /// Untagged items realize half-width Latin delimiters (today's behavior).
    #[default]
    Latin,
    /// Untagged items realize full-width CJK delimiters (e.g. GB/T 7714).
    Cjk,
}

/// Named table used to realize semantic structural punctuation.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum PunctuationWidth {
    /// Use ASCII/Narrow punctuation for every item.
    Half,
    /// Use full-width punctuation for every semantic mark.
    Full,
    /// Use full-width punctuation except ASCII periods and square brackets.
    Mixed,
    /// Use full-width punctuation for CJK items and narrow punctuation otherwise.
    Bylan,
}

impl RealizationDefault {
    /// Returns `true` for the default variant (`Latin`), used for skip-serializing.
    pub fn is_latin(&self) -> bool {
        matches!(self, RealizationDefault::Latin)
    }
}

/// Custom deserializer for [`MultilingualMode`].
///
/// Unit variants are accepted as plain strings (`"primary"`, `"transliterated"`, etc.).
/// The `Pattern` variant is accepted as a single-key map `{pattern: [...]}`.
///
/// A hand-written `deserialize_any` visitor is used instead of serde's derived
/// `deserialize_enum` because `serde_yaml` cannot pass enum-variant input through
/// an outer `#[serde(untagged)]` wrapper — the standard derive would fail with
/// *"untagged and internally tagged enums do not support enum input"* when a
/// serialized `Pattern` value is round-tripped through [`crate::presets::MultilingualConfigEntry`].
impl<'de> de::Deserialize<'de> for MultilingualMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ModeVisitor;

        impl<'de> de::Visitor<'de> for ModeVisitor {
            type Value = MultilingualMode;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "a multilingual mode string (\"primary\", \"transliterated\", \
                     \"translated\", \"combined\") or a pattern object {{pattern: [...]}}"
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "primary" => Ok(MultilingualMode::Primary),
                    "transliterated" => Ok(MultilingualMode::Transliterated),
                    "translated" => Ok(MultilingualMode::Translated),
                    "combined" => Ok(MultilingualMode::Combined),
                    _ => Err(E::unknown_variant(
                        v,
                        &[
                            "primary",
                            "transliterated",
                            "translated",
                            "combined",
                            "pattern",
                        ],
                    )),
                }
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expected \"pattern\" key, got empty map"))?;
                if key != "pattern" {
                    return Err(de::Error::unknown_field(&key, &["pattern"]));
                }
                let segments: Vec<MultilingualSegment> = map.next_value()?;
                if map.next_key::<String>()?.is_some() {
                    return Err(de::Error::custom("unexpected extra key in pattern object"));
                }
                Ok(MultilingualMode::Pattern(segments))
            }

            /// serde_yaml serialises `{pattern: [...]}` as an externally-tagged enum
            /// (not a plain map), so we also need to handle enum access.
            fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
                use de::VariantAccess as _;
                let (variant, access): (String, _) = data.variant()?;
                if variant == "pattern" {
                    let segments: Vec<MultilingualSegment> = access.newtype_variant()?;
                    Ok(MultilingualMode::Pattern(segments))
                } else {
                    Err(de::Error::unknown_variant(
                        &variant,
                        &[
                            "primary",
                            "transliterated",
                            "translated",
                            "combined",
                            "pattern",
                        ],
                    ))
                }
            }
        }

        deserializer.deserialize_any(ModeVisitor)
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
mod term_locale_tests {
    use super::*;

    #[test]
    fn term_locale_item_round_trips_through_yaml() {
        let config: MultilingualConfig = serde_yaml::from_str("term-locale: item").unwrap();
        assert_eq!(config.term_locale, TermLocale::Item);

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("term-locale: item"));

        let round_tripped: MultilingualConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(round_tripped, config);
    }

    #[test]
    fn absent_term_locale_field_defaults_to_style() {
        let config: MultilingualConfig = serde_yaml::from_str("{}").unwrap();
        assert_eq!(config.term_locale, TermLocale::Style);
    }

    #[test]
    fn default_term_locale_is_omitted_on_serialize() {
        let config = MultilingualConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(
            !yaml.contains("term-locale"),
            "term-locale: style is today's byte-identical default and must be omitted: {yaml}"
        );
    }
}
