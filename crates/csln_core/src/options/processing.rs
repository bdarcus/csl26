/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Label style preset conventions.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LabelPreset {
    /// biblatex alphabetic / BibTeX alpha.bst: up to 4 authors, "+" marker, 2-digit year.
    #[default]
    Alpha,
    /// DIN 1505-2: up to 3 authors, no et-al marker, 2-digit year.
    Din,
    /// American Mathematical Society: same algorithm as Alpha, sorted by citation-number.
    Ams,
}

/// Resolved label generation parameters after applying preset defaults.
#[derive(Debug, Clone)]
pub struct LabelParams {
    pub single_author_chars: u8,
    pub multi_author_chars: u8,
    pub et_al_min: u8,
    pub et_al_marker: String,
    pub year_digits: u8,
}

/// Configuration for label citation mode.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct LabelConfig {
    /// Preset that determines default parameters.
    #[serde(default)]
    pub preset: LabelPreset,
    /// Chars taken from single author's family name. Preset default: 3 (Alpha/Ams), 4 (Din).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_author_chars: Option<u8>,
    /// Chars per author family name when 2+ authors. Preset default: 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_author_chars: Option<u8>,
    /// Max authors before truncation. Alpha/Ams default: 4, Din default: 3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al_min: Option<u8>,
    /// Suffix appended when truncated. Alpha/Ams default: "+", Din default: "".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al_marker: Option<String>,
    /// Year digits: 2 or 4. Preset default: 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_digits: Option<u8>,
}

impl LabelConfig {
    /// Resolve effective parameters by merging preset defaults with overrides.
    pub fn effective_params(&self) -> LabelParams {
        let (
            default_single_author_chars,
            default_multi_author_chars,
            default_et_al_min,
            default_marker,
        ) = match self.preset {
            LabelPreset::Alpha | LabelPreset::Ams => (3u8, 1u8, 4u8, "+".to_string()),
            LabelPreset::Din => (4u8, 1u8, 3u8, String::new()),
        };
        LabelParams {
            single_author_chars: self
                .single_author_chars
                .unwrap_or(default_single_author_chars),
            multi_author_chars: self
                .multi_author_chars
                .unwrap_or(default_multi_author_chars),
            et_al_min: self.et_al_min.unwrap_or(default_et_al_min),
            et_al_marker: self.et_al_marker.clone().unwrap_or(default_marker),
            year_digits: self.year_digits.unwrap_or(2),
        }
    }
}

/// Processing mode for citation/bibliography generation.
///
/// Can be specified as:
/// - A string: "author-date", "numeric", "note", or "label"
/// - A label config map: { label: { preset: din } }
/// - A custom config map: { sort: ..., group: ..., disambiguate: ... }
#[derive(Debug, Default, PartialEq, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Processing {
    #[default]
    AuthorDate,
    Numeric,
    Note,
    Label(LabelConfig),
    Custom(ProcessingCustom),
}

/// Custom processing configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
            Processing::Label(_) => ProcessingCustom {
                sort: None,
                group: None,
                disambiguate: Some(Disambiguation {
                    names: false,
                    add_givenname: false,
                    year_suffix: true,
                }),
            },
            Processing::Custom(custom) => custom.clone(),
        }
    }
}

impl<'de> Deserialize<'de> for Processing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct ProcessingVisitor;

        impl<'de> Visitor<'de> for ProcessingVisitor {
            type Value = Processing;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a processing mode string or map")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Processing, E> {
                match v {
                    "author-date" => Ok(Processing::AuthorDate),
                    "numeric" => Ok(Processing::Numeric),
                    "note" => Ok(Processing::Note),
                    "label" => Ok(Processing::Label(LabelConfig::default())),
                    other => Err(E::unknown_variant(
                        other,
                        &["author-date", "numeric", "note", "label"],
                    )),
                }
            }

            fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<Processing, A::Error> {
                use serde::de::VariantAccess;
                let (variant, access) = data.variant::<String>()?;
                match variant.as_str() {
                    "custom" => {
                        let custom: ProcessingCustom = access.newtype_variant()?;
                        Ok(Processing::Custom(custom))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["author-date", "numeric", "note", "label", "custom"],
                    )),
                }
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Processing, A::Error> {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::invalid_length(0, &"1"))?;
                match key.as_str() {
                    "label" => {
                        let config: LabelConfig = map.next_value()?;
                        Ok(Processing::Label(config))
                    }
                    "sort" | "group" | "disambiguate" => {
                        // This is a custom processing config
                        // We need to deserialize the whole map as ProcessingCustom
                        // Unfortunately we can't easily re-parse from the middle of map access.
                        // Instead, collect fields and build manually
                        let mut sort = None;
                        let mut group = None;
                        let mut disambiguate = None;

                        // Handle the first key we already read
                        match key.as_str() {
                            "sort" => sort = Some(map.next_value()?),
                            "group" => group = Some(map.next_value()?),
                            "disambiguate" => disambiguate = Some(map.next_value()?),
                            _ => {
                                return Err(de::Error::unknown_field(
                                    &key,
                                    &["sort", "group", "disambiguate"],
                                ));
                            }
                        }

                        // Read remaining keys
                        while let Some(k) = map.next_key::<String>()? {
                            match k.as_str() {
                                "sort" => sort = Some(map.next_value()?),
                                "group" => group = Some(map.next_value()?),
                                "disambiguate" => disambiguate = Some(map.next_value()?),
                                other => {
                                    return Err(de::Error::unknown_field(
                                        other,
                                        &["sort", "group", "disambiguate"],
                                    ));
                                }
                            }
                        }

                        Ok(Processing::Custom(ProcessingCustom {
                            sort,
                            group,
                            disambiguate,
                        }))
                    }
                    other => Err(de::Error::unknown_field(
                        other,
                        &["label", "sort", "group", "disambiguate"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(ProcessingVisitor)
    }
}

/// Disambiguation settings.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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

/// Sorting configuration.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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

/// Grouping configuration for bibliography.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Group {
    pub template: Vec<SortKey>,
}
