/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::reference::types::RefDate;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An EDTF date value, with an optional opaque note.
///
/// `value` is the canonical EDTF string driving all date computation
/// (sorting, disambiguation, fallback selection). `note` is optional,
/// uninterpreted display text a data producer supplies alongside the date —
/// e.g. source-calendar wording such as `民国三十六年` next to a Gregorian
/// `value` of `"1947"`. Citum never parses, converts, or validates `note`;
/// see [`docs/specs/CALENDAR_DATE_ANNOTATIONS.md`](../../../../docs/specs/CALENDAR_DATE_ANNOTATIONS.md).
///
/// The wire format is backward compatible: a bare EDTF string (`"1947"`)
/// deserializes with `note: None` and serializes back to the same bare
/// string. Supplying a `note` requires the mapping form
/// `{ value: "1947", note: "..." }`, and only then does serialization emit
/// the mapping form.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct DateValue {
    /// The EDTF value.
    pub value: String,
    /// Optional opaque, uninterpreted text alongside the date. Preserved
    /// verbatim; never parsed, converted, or validated.
    #[cfg_attr(feature = "bindings", specta(optional))]
    pub note: Option<String>,
}

impl DateValue {
    /// Construct a `DateValue` with no note, from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            note: None,
        }
    }

    /// Check if the date value is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Parse the string as an EDTF date etc, or return the string as a literal.
    pub fn parse(&self) -> RefDate {
        match self.value.parse::<citum_edtf::Edtf>() {
            Ok(edtf) => RefDate::Edtf(edtf),
            Err(_) => RefDate::Literal(self.value.clone()),
        }
    }

    /// Extract the year from the date.
    pub fn year(&self) -> String {
        match self.parse() {
            RefDate::Edtf(edtf) => edtf.year().to_string(),
            RefDate::Literal(_) => String::new(),
        }
    }

    /// Extract the month from the date.
    pub fn month(&self) -> Option<u32> {
        match self.parse() {
            RefDate::Edtf(edtf) => edtf.month(),
            RefDate::Literal(_) => None,
        }
    }

    /// Extract the day from the date.
    pub fn day(&self) -> Option<u32> {
        match self.parse() {
            RefDate::Edtf(edtf) => edtf.day().filter(|&d| d > 0),
            RefDate::Literal(_) => None,
        }
    }

    /// Extract (year, month, day) from a single EDTF parse, avoiding the
    /// separate re-parse each of `.year()`, `.month()`, and `.day()` would
    /// otherwise incur. `None` for an unparseable (`Literal`) date, matching
    /// `.year()`'s empty-string / `.month()`'s and `.day()`'s `None`
    /// behavior for that case.
    pub fn date_parts(&self) -> Option<(i64, Option<u32>, Option<u32>)> {
        match self.parse() {
            RefDate::Edtf(edtf) => Some((edtf.year(), edtf.month(), edtf.day().filter(|&d| d > 0))),
            RefDate::Literal(_) => None,
        }
    }

    /// Check if the date is uncertain (has "?" qualifier).
    pub fn is_uncertain(&self) -> bool {
        self.value.contains('?')
    }

    /// Check if the date is approximate (has "~" qualifier).
    pub fn is_approximate(&self) -> bool {
        self.value.contains('~')
    }

    /// Check if the date is a range (interval).
    pub fn is_range(&self) -> bool {
        matches!(self.parse(), RefDate::Edtf(edtf) if edtf.is_range())
    }

    /// Check if the range is open-ended (ends with "..").
    pub fn is_open_range(&self) -> bool {
        matches!(self.parse(), RefDate::Edtf(edtf) if edtf.is_open_range())
    }

    /// Extract the time component from the date, if present.
    pub fn time(&self) -> Option<citum_edtf::Time> {
        match self.parse() {
            RefDate::Edtf(edtf) => edtf.time(),
            _ => None,
        }
    }

    /// Check if the date has a time component.
    pub fn has_time(&self) -> bool {
        self.time().is_some()
    }
}

impl fmt::Display for DateValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for DateValue {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for DateValue {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Wire representation used only to deserialize [`DateValue`]: either a bare
/// EDTF string, or the explicit `{ value, note }` mapping. Unknown mapping
/// keys are rejected.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DateValueStructured {
    value: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DateValueRepr {
    Scalar(String),
    Structured(DateValueStructured),
}

impl<'de> Deserialize<'de> for DateValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match DateValueRepr::deserialize(deserializer)? {
            DateValueRepr::Scalar(value) => DateValue { value, note: None },
            DateValueRepr::Structured(DateValueStructured { value, note }) => {
                DateValue { value, note }
            }
        })
    }
}

impl Serialize for DateValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.note {
            None => serializer.serialize_str(&self.value),
            Some(note) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("value", &self.value)?;
                map.serialize_entry("note", note)?;
                map.end()
            }
        }
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for DateValue {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DateValue".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let scalar_schema = generator.subschema_for::<String>();
        let structured_schema = schemars::json_schema!({
            "type": "object",
            "properties": {
                "value": generator.subschema_for::<String>(),
                "note": generator.subschema_for::<Option<String>>()
            },
            "required": ["value"],
            "additionalProperties": false
        });
        schemars::json_schema!({
            "oneOf": [scalar_schema, structured_schema]
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "Panicking is acceptable in tests.")]
mod tests {
    use super::*;

    #[test]
    fn scalar_input_round_trips_byte_identically() {
        let json = r#""1947""#;
        let date: DateValue = serde_json::from_str(json).unwrap();
        assert_eq!(date, DateValue::new("1947"));
        assert_eq!(serde_json::to_string(&date).unwrap(), json);
    }

    #[test]
    fn mapping_form_parses_value_and_note() {
        let json = r#"{"value":"1947","note":"民国三十六年"}"#;
        let date: DateValue = serde_json::from_str(json).unwrap();
        assert_eq!(date.value, "1947");
        assert_eq!(date.note.as_deref(), Some("民国三十六年"));
    }

    #[test]
    fn mapping_form_without_note_defaults_to_none() {
        let json = r#"{"value":"1947"}"#;
        let date: DateValue = serde_json::from_str(json).unwrap();
        assert_eq!(date, DateValue::new("1947"));
    }

    #[test]
    fn mapping_form_rejects_unknown_fields() {
        let json = r#"{"value":"1947","note":"民国三十六年","calendar":"minguo"}"#;
        let err = serde_json::from_str::<DateValue>(json).unwrap_err();
        assert!(
            err.to_string()
                .contains("did not match any variant of untagged enum")
        );
    }

    #[test]
    fn mapping_form_requires_value() {
        let json = r#"{"note":"民国三十六年"}"#;
        assert!(serde_json::from_str::<DateValue>(json).is_err());
    }

    #[test]
    fn note_present_serializes_as_mapping_not_scalar() {
        let date = DateValue {
            value: "1947".to_string(),
            note: Some("民国三十六年".to_string()),
        };
        let json = serde_json::to_string(&date).unwrap();
        assert_eq!(json, r#"{"value":"1947","note":"民国三十六年"}"#);

        let round_tripped: DateValue = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, date);
    }

    #[test]
    fn note_is_ignored_by_value_oriented_accessors() {
        // The note must never influence anything computed off the date's
        // canonical value: sorting, disambiguation, and fallback selection
        // all read `value`, never `note`. See CALENDAR_DATE_ANNOTATIONS.md.
        let annotated = DateValue {
            value: "1947".to_string(),
            note: Some("民国三十六年".to_string()),
        };
        let plain = DateValue::new("1947");
        assert_eq!(annotated.value, plain.value);
        assert_eq!(annotated.year(), plain.year());
        assert_eq!(annotated.is_empty(), plain.is_empty());
    }
}
