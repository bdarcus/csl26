/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::locale::SubYearCode;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_yaml::{Mapping, Value};
#[cfg(feature = "schema")]
use std::borrow::Cow;
use std::collections::HashMap;

/// Raw locale format for YAML parsing.
/// This is a simpler format that uses string keys for terms.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RawLocale {
    /// The locale identifier (e.g., "en-US", "de-DE").
    pub locale: String,
    /// Date-related terms.
    #[serde(default)]
    pub dates: RawDateTerms,
    /// Role terms keyed by role name.
    #[serde(default)]
    pub roles: HashMap<String, RawRoleTerm>,
    /// General terms keyed by term name.
    #[serde(default)]
    pub terms: HashMap<String, RawTermValue>,
    /// Locator terms keyed by locator name.
    #[serde(default)]
    pub locators: HashMap<String, RawLocatorTerm>,
    /// Schema version. Absent or "1" uses the legacy term-map path.
    /// "2" activates the new messages/dateFormats/grammarOptions path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale_schema_version: Option<String>,
    /// Runtime evaluation options (message syntax, evaluator hints).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<crate::locale::types::EvaluationConfig>,
    /// ICU Message Format 1 messages keyed by message ID (v2 locales only).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub messages: HashMap<String, String>,
    /// Named date format presets: symbolic name → CLDR date pattern (v2 locales only).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub date_formats: HashMap<String, String>,
    /// Locale-level number formatting options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_formats: Option<crate::locale::types::NumberFormats>,
    /// Grammar toggles that vary by language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grammar_options: Option<crate::locale::types::GrammarOptions>,
    /// Partial semantic punctuation realization table owned by this locale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub punctuation_realization: Option<crate::options::PunctuationRealization>,
    /// Backwards-compatibility aliases: old CSL term key → new message ID.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub legacy_term_aliases: HashMap<String, String>,
    /// Vocabulary maps for genre and medium display text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vocab: Option<RawVocab>,
}

/// Raw vocab maps for genre and medium display text.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RawVocab {
    /// Genre canonical key → display string.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub genre: HashMap<String, String>,
    /// Medium canonical key → display string.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub medium: HashMap<String, String>,
}

/// Raw date terms for YAML parsing.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RawDateTerms {
    /// Localized month names.
    #[serde(default)]
    pub months: RawMonthNames,
    /// Localized season names, either the legacy display-order sequence
    /// (`[Spring, Summer, Autumn, Winter]`) or the canonical EDTF-season-code
    /// map (`{21: Spring, 22: Summer, ...}`). See
    /// `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
    #[serde(default)]
    pub seasons: RawDateNameField,
    /// Localized term for uncertain dates.
    #[serde(default)]
    pub uncertainty_term: Option<String>,
    /// Localized term for open-ended date ranges.
    #[serde(default)]
    pub open_ended_term: Option<String>,
    /// Localized ante meridiem marker.
    #[serde(default)]
    pub am: Option<String>,
    /// Localized post meridiem marker.
    #[serde(default)]
    pub pm: Option<String>,
    /// Localized label for UTC.
    #[serde(default)]
    pub timezone_utc: Option<String>,
    /// Localized era suffix for year zero and negative years.
    #[serde(default)]
    pub before_era: Option<String>,
    /// Localized era suffix for positive years in BC/AD profile (e.g., "AD").
    #[serde(default)]
    pub ad: Option<String>,
    /// Localized era suffix for negative years in BC/AD profile (e.g., "BC").
    #[serde(default)]
    pub bc: Option<String>,
    /// Localized era suffix for negative years in BCE/CE profile (e.g., "BCE").
    #[serde(default)]
    pub bce: Option<String>,
    /// Localized era suffix for positive years in BCE/CE profile (e.g., "CE").
    #[serde(default)]
    pub ce: Option<String>,
}

/// Raw month names for YAML parsing.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RawMonthNames {
    /// Full month names, sequence or EDTF-month-code-keyed map.
    #[serde(default)]
    pub long: RawDateNameField,
    /// Abbreviated month names, sequence or EDTF-month-code-keyed map.
    #[serde(default)]
    pub short: RawDateNameField,
}

/// One date-name table (a set of `months.long`, `months.short`, or
/// `seasons`), in either the legacy ordered-sequence form or the canonical
/// EDTF-sub-year-code-keyed map form.
///
/// Both forms parse; [`RawDateNameField::into_map`] canonicalizes a sequence
/// into the map, assigning sequence index `i` (0-based) to code
/// `start_code + i`. This is the only place the legacy shape is known — the
/// runtime [`crate::locale::Locale`] only ever sees the map form. See
/// `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum RawDateNameField {
    /// Legacy display-order sequence (e.g. `[January, February, ...]`).
    Sequence(Vec<String>),
    /// Canonical EDTF-sub-year-code-keyed map (e.g. `{1: January, ...}`).
    Keyed(std::collections::BTreeMap<crate::locale::types::SubYearCode, String>),
}

impl Default for RawDateNameField {
    fn default() -> Self {
        Self::Sequence(Vec::new())
    }
}

impl RawDateNameField {
    /// Canonicalize into an EDTF-sub-year-code-keyed map.
    ///
    /// A [`Self::Sequence`] entry at 0-based index `i` is assigned code
    /// `start_code + i`; entries whose resulting code falls outside the
    /// valid month/season range are dropped rather than panicking, since
    /// this is reachable from untrusted locale-file input. A
    /// [`Self::Keyed`] value passes through unchanged.
    #[must_use]
    pub fn into_map(
        self,
        start_code: u8,
    ) -> std::collections::BTreeMap<crate::locale::types::SubYearCode, String> {
        match self {
            Self::Sequence(names) => names
                .into_iter()
                .enumerate()
                .filter_map(|(i, name)| {
                    let offset = u8::try_from(i).ok()?;
                    let code = start_code.checked_add(offset)?;
                    crate::locale::types::SubYearCode::new(code).map(|c| (c, name))
                })
                .collect(),
            Self::Keyed(map) => map,
        }
    }
}

/// Raw role term with form-keyed values.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RawRoleTerm {
    /// Long-form role term.
    #[serde(default)]
    pub long: Option<RawTermValue>,
    /// Short-form role term.
    #[serde(default)]
    pub short: Option<RawTermValue>,
    /// Verb-form role term.
    #[serde(default)]
    pub verb: Option<RawTermValue>,
    /// Short verb-form role term.
    #[serde(default, rename = "verb-short")]
    pub verb_short: Option<RawTermValue>,
}

/// Raw locator term with optional lexical gender.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RawLocatorTerm {
    /// Long-form locator term.
    #[serde(default)]
    pub long: Option<RawTermValue>,
    /// Short-form locator term.
    #[serde(default)]
    pub short: Option<RawTermValue>,
    /// Symbol-form locator term.
    #[serde(default)]
    pub symbol: Option<RawTermValue>,
    /// Lexical gender used for noun agreement.
    #[serde(default)]
    pub gender: Option<crate::locale::types::GrammaticalGender>,
}

/// A term value that can be a simple string or have singular/plural forms.
#[derive(Debug, Clone, Serialize)]
pub enum RawTermValue {
    /// Simple string value.
    Simple(String),
    /// Singular/plural forms.
    SingularPlural {
        /// Singular form of the term.
        singular: RawGenderedString,
        /// Plural form of the term.
        plural: RawGenderedString,
    },
    /// Gender-specific values.
    Gendered {
        /// Masculine form.
        #[serde(default)]
        masculine: Option<String>,
        /// Feminine form.
        #[serde(default)]
        feminine: Option<String>,
        /// Neuter form.
        #[serde(default)]
        neuter: Option<String>,
        /// Common or shared form.
        #[serde(default)]
        common: Option<String>,
    },
    /// Form-keyed value (for terms with long/short forms).
    Forms(HashMap<String, RawTermValue>),
}

/// A raw string that may include gender-specific variants.
#[derive(Debug, Clone, Serialize)]
pub enum RawGenderedString {
    /// Plain string value.
    Simple(String),
    /// Gender-specific values.
    Gendered {
        /// Masculine form.
        #[serde(default)]
        masculine: Option<String>,
        /// Feminine form.
        #[serde(default)]
        feminine: Option<String>,
        /// Neuter form.
        #[serde(default)]
        neuter: Option<String>,
        /// Common or shared form.
        #[serde(default)]
        common: Option<String>,
    },
}

impl Default for RawTermValue {
    fn default() -> Self {
        RawTermValue::Simple(String::new())
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for RawGenderedString {
    fn schema_name() -> Cow<'static, str> {
        "RawGenderedString".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "description": "A raw string that may include gender-specific variants.",
            "anyOf": [
                {
                    "description": "Plain string value.",
                    "type": "string"
                },
                {
                    "description": "Gender-specific values.",
                    "type": "object",
                    "properties": gender_slot_schema_properties(),
                    "additionalProperties": false,
                    "minProperties": 1
                }
            ]
        })
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for RawTermValue {
    fn schema_name() -> Cow<'static, str> {
        "RawTermValue".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // Register RawGenderedString so it appears in $defs
        generator.subschema_for::<RawGenderedString>();

        schemars::json_schema!({
            "description": "A term value that can be a simple string or have singular/plural forms.",
            "anyOf": [
                {
                    "description": "Simple string value.",
                    "type": "string"
                },
                {
                    "description": "Singular/plural forms.",
                    "type": "object",
                    "properties": {
                        "singular": { "$ref": "#/$defs/RawGenderedString" },
                        "plural": { "$ref": "#/$defs/RawGenderedString" }
                    },
                    "required": ["singular", "plural"],
                    "additionalProperties": false
                },
                {
                    "description": "Gender-specific values.",
                    "type": "object",
                    "properties": gender_slot_schema_properties(),
                    "additionalProperties": false,
                    "minProperties": 1
                },
                {
                    "description": "Form-keyed value (for terms with long/short or nested forms).",
                    "type": "object",
                    "additionalProperties": {
                        "$ref": "#/$defs/RawTermValue"
                    }
                }
            ]
        })
    }
}

impl<'de> Deserialize<'de> for RawTermValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Self::from_value(value).map_err(D::Error::custom)
    }
}

impl<'de> Deserialize<'de> for RawGenderedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Self::from_value(value).map_err(D::Error::custom)
    }
}

impl RawTermValue {
    /// Get the simple string value.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            RawTermValue::Simple(s) => Some(s),
            _ => None,
        }
    }

    fn from_value(value: Value) -> Result<Self, String> {
        match value {
            Value::String(s) => Ok(Self::Simple(s)),
            Value::Mapping(map) => {
                if let Some((singular, plural)) = parse_singular_plural_map(&map)? {
                    return Ok(Self::SingularPlural { singular, plural });
                }

                if let Some(gendered) = parse_gendered_map(&map)? {
                    return Ok(gendered);
                }

                let forms = map_to_term_values(map)?;
                Ok(Self::Forms(forms))
            }
            other => Err(format!(
                "expected string or mapping for locale term, found {}",
                value_kind(&other)
            )),
        }
    }
}

impl RawGenderedString {
    fn from_value(value: Value) -> Result<Self, String> {
        match value {
            Value::String(s) => Ok(Self::Simple(s)),
            Value::Mapping(map) => parse_gendered_string_map(&map)?
                .ok_or_else(|| "expected string or gender-specific mapping".to_string()),
            other => Err(format!(
                "expected string or mapping for gendered locale string, found {}",
                value_kind(&other)
            )),
        }
    }
}

fn parse_singular_plural_map(
    map: &Mapping,
) -> Result<Option<(RawGenderedString, RawGenderedString)>, String> {
    if !contains_only_keys(map, &["singular", "plural"])? {
        return Ok(None);
    }

    if map.is_empty() {
        return Ok(None);
    }

    let Some(singular) = map.get(Value::String("singular".to_string())) else {
        return Ok(None);
    };
    let Some(plural) = map.get(Value::String("plural".to_string())) else {
        return Ok(None);
    };

    Ok(Some((
        RawGenderedString::from_value(singular.clone())?,
        RawGenderedString::from_value(plural.clone())?,
    )))
}

fn parse_gendered_map(map: &Mapping) -> Result<Option<RawTermValue>, String> {
    parse_gender_slots(map).map(|slots| {
        slots.map(
            |(masculine, feminine, neuter, common)| RawTermValue::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            },
        )
    })
}

fn parse_gendered_string_map(map: &Mapping) -> Result<Option<RawGenderedString>, String> {
    parse_gender_slots(map).map(|slots| {
        slots.map(
            |(masculine, feminine, neuter, common)| RawGenderedString::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            },
        )
    })
}

type GenderSlots = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn parse_gender_slots(map: &Mapping) -> Result<Option<GenderSlots>, String> {
    let (has_gender_key, has_non_gender_key) = inspect_gender_keys(map)?;
    if !has_gender_key {
        return Ok(None);
    }
    if has_non_gender_key {
        return Err("gendered locale terms cannot mix gender keys with other keys".to_string());
    }

    let masculine = map
        .get(Value::String("masculine".to_string()))
        .map(parse_optional_string_value)
        .transpose()?
        .flatten();
    let feminine = map
        .get(Value::String("feminine".to_string()))
        .map(parse_optional_string_value)
        .transpose()?
        .flatten();
    let neuter = map
        .get(Value::String("neuter".to_string()))
        .map(parse_optional_string_value)
        .transpose()?
        .flatten();
    let common = map
        .get(Value::String("common".to_string()))
        .map(parse_optional_string_value)
        .transpose()?
        .flatten();

    Ok(Some((masculine, feminine, neuter, common)))
}

fn contains_only_keys(map: &Mapping, allowed: &[&str]) -> Result<bool, String> {
    for key in map.keys() {
        let Value::String(key) = key else {
            return Err("locale term keys must be strings".to_string());
        };

        if !allowed.contains(&key.as_str()) {
            return Ok(false);
        }
    }

    Ok(true)
}

fn map_to_term_values(map: Mapping) -> Result<HashMap<String, RawTermValue>, String> {
    map.into_iter()
        .map(|(key, value)| {
            let Value::String(key) = key else {
                return Err("locale term keys must be strings".to_string());
            };
            Ok((key, RawTermValue::from_value(value)?))
        })
        .collect()
}

fn parse_optional_string_value(value: &Value) -> Result<Option<String>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(value) => Ok(Some(value.clone())),
        other => Err(format!(
            "expected string in gendered locale term, found {}",
            value_kind(other)
        )),
    }
}

fn inspect_gender_keys(map: &Mapping) -> Result<(bool, bool), String> {
    let mut has_gender_key = false;
    let mut has_non_gender_key = false;

    for key in map.keys() {
        let Value::String(key) = key else {
            return Err("locale term keys must be strings".to_string());
        };

        match key.as_str() {
            "masculine" | "feminine" | "neuter" | "common" => has_gender_key = true,
            _ => has_non_gender_key = true,
        }
    }

    Ok((has_gender_key, has_non_gender_key))
}

#[cfg(feature = "schema")]
fn gender_slot_schema_properties() -> serde_json::Value {
    serde_json::json!({
        "masculine": { "type": ["string", "null"] },
        "feminine": { "type": ["string", "null"] },
        "neuter": { "type": ["string", "null"] },
        "common": { "type": ["string", "null"] }
    })
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged value",
    }
}

/// Raw locale override format for YAML parsing.
///
/// Mirrors [`super::types::LocaleOverride`] for deserialization from style-level
/// locale override files.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default)]
pub struct RawLocaleOverride {
    /// Message IDs to replace in the base locale.
    pub messages: HashMap<String, String>,
    /// If present, replaces the entire grammar-options block.
    pub grammar_options: Option<crate::locale::types::GrammarOptions>,
    /// Additional or replacement legacy term aliases.
    pub legacy_term_aliases: HashMap<String, String>,
    /// Sparse month/season name replacements.
    #[serde(default)]
    pub dates: RawDateNameOverride,
}

/// Raw sparse month/season name overrides, mirroring
/// [`super::types::DateNameOverride`] for deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default)]
pub struct RawDateNameOverride {
    /// Month name replacements.
    pub months: RawMonthNames,
    /// Season name replacements.
    pub seasons: RawDateNameField,
}

impl From<RawLocaleOverride> for super::types::LocaleOverride {
    fn from(raw: RawLocaleOverride) -> Self {
        super::types::LocaleOverride {
            messages: raw.messages,
            grammar_options: raw.grammar_options,
            legacy_term_aliases: raw.legacy_term_aliases,
            dates: super::types::DateNameOverride {
                months: super::types::MonthNames {
                    long: raw.dates.months.long.into_map(SubYearCode::MIN_MONTH),
                    short: raw.dates.months.short.into_map(SubYearCode::MIN_MONTH),
                },
                seasons: raw.dates.seasons.into_map(SubYearCode::MIN_SEASON),
            },
        }
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
    use super::{RawDateNameField, RawGenderedString, RawTermValue};
    #[cfg(feature = "schema")]
    use crate::locale::RawLocale;
    use crate::locale::SubYearCode;

    /// The legacy ordered sequence canonicalizes to the same codes the map
    /// form would use directly: month index `i` -> code `i+1`, season
    /// index `i` -> code `21+i`.
    #[test]
    fn raw_date_name_field_sequence_canonicalizes_to_expected_codes() {
        let months = RawDateNameField::Sequence(vec!["January".into(), "February".into()]);
        let map = months.into_map(SubYearCode::MIN_MONTH);
        assert_eq!(
            map.get(&SubYearCode::new(1).expect("valid month code")),
            Some(&"January".to_string())
        );
        assert_eq!(
            map.get(&SubYearCode::new(2).expect("valid month code")),
            Some(&"February".to_string())
        );

        let seasons = RawDateNameField::Sequence(vec!["Spring".into(), "Summer".into()]);
        let season_map = seasons.into_map(SubYearCode::MIN_SEASON);
        assert_eq!(
            season_map.get(&SubYearCode::new(21).expect("valid season code")),
            Some(&"Spring".to_string())
        );
        assert_eq!(
            season_map.get(&SubYearCode::new(22).expect("valid season code")),
            Some(&"Summer".to_string())
        );
    }

    /// The canonical keyed map form passes through `into_map` unchanged.
    #[test]
    fn raw_date_name_field_keyed_passes_through() {
        let keyed = RawDateNameField::Keyed(
            [(
                SubYearCode::new(7).expect("valid month code"),
                "Jul.".to_string(),
            )]
            .into(),
        );
        let map = keyed.into_map(SubYearCode::MIN_MONTH);
        assert_eq!(
            map.get(&SubYearCode::new(7).expect("valid month code")),
            Some(&"Jul.".to_string())
        );
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_gender_slots_accept_explicit_null_values() {
        let parsed: RawTermValue = serde_yaml::from_str(
            r#"
masculine: editor
feminine: editora
common: null
"#,
        )
        .expect("gendered term with null slot should parse");

        match parsed {
            RawTermValue::Gendered {
                masculine,
                feminine,
                common,
                ..
            } => {
                assert_eq!(masculine.as_deref(), Some("editor"));
                assert_eq!(feminine.as_deref(), Some("editora"));
                assert_eq!(common, None);
            }
            other => panic!("expected gendered term, got {other:?}"),
        }
    }

    #[test]
    fn test_all_null_gender_slots_still_parse() {
        let parsed: RawGenderedString = serde_yaml::from_str(
            r#"
masculine: null
feminine: null
common: null
"#,
        )
        .expect("all-null gender map should parse");

        match parsed {
            RawGenderedString::Gendered {
                masculine,
                feminine,
                common,
                ..
            } => {
                assert!(masculine.is_none());
                assert!(feminine.is_none());
                assert!(common.is_none());
            }
            other => panic!("expected gendered string, got {other:?}"),
        }
    }

    #[test]
    fn test_malformed_gender_map_reports_targeted_error() {
        let error = serde_yaml::from_str::<RawTermValue>(
            r#"
masculine: editor
femine: editora
"#,
        )
        .expect_err("mixed gender-like map should fail");

        assert!(
            error
                .to_string()
                .contains("gendered locale terms cannot mix gender keys")
        );
    }

    #[cfg(feature = "schema")]
    #[test]
    fn test_raw_term_schema_remains_untagged() {
        let schema = schemars::schema_for!(RawLocale);
        let schema_text = serde_json::to_string(&schema).expect("schema should serialize");

        assert!(schema_text.contains("\"RawTermValue\""));
        assert!(schema_text.contains("\"type\":\"string\""));
        assert!(schema_text.contains("\"$ref\":\"#/$defs/RawGenderedString\""));
        assert!(!schema_text.contains("\"Simple\""));
    }
}
