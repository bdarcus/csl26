/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Processing mode and citation/bibliography rendering options.
//!
//! This module defines the processing modes (author-date, numeric, note, label, custom) that
//! determine how citations and bibliographies are sorted, grouped, and disambiguated. Each
//! mode provides default configurations for sorting and disambiguation strategies.

/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::presets::SortPreset;

const PROCESSING_STRING_VARIANTS: &[&str] = &[
    "author-date",
    "author-date-givenname",
    "author-date-names",
    "author-date-full",
    "numeric",
    "note",
    "label",
];

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
    /// CSL/citeproc alphabetic labels used by American Mathematical Society styles.
    Ams,
}

/// Resolved label generation parameters after applying preset defaults.
///
/// Stores the resolved (effective) parameters for label citation mode, combining
/// preset defaults with any user-specified overrides from `LabelConfig`.
#[derive(Debug, Clone)]
pub struct LabelParams {
    /// Number of characters from a single author's family name.
    pub single_author_chars: u8,
    /// Number of characters per author when multiple authors are present.
    pub multi_author_chars: u8,
    /// Maximum number of authors before truncation (et-al).
    pub et_al_min: u8,
    /// Suffix to append when authors are truncated (e.g., "+").
    pub et_al_marker: String,
    /// Number of names to show in et-al truncation.
    pub et_al_names: u8,
    /// Number of year digits to use (typically 2 or 4).
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
    /// Chars taken from single author's family name. Preset default: 3 (Alpha), 4 (Ams/Din).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_author_chars: Option<u8>,
    /// Chars per author family name when 2+ authors. Preset default: 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_author_chars: Option<u8>,
    /// Max authors before truncation. Alpha default: 4, Ams default: 5, Din default: 3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al_min: Option<u8>,
    /// Suffix appended when truncated. Alpha default: "+", Ams/Din default: "".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al_marker: Option<String>,
    /// Names shown when truncated (et-al). Alpha default: 3, Ams default: 4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al_names: Option<u8>,
    /// Year digits: 2 or 4. Preset default: 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_digits: Option<u8>,
}

impl LabelConfig {
    /// Resolve effective parameters by merging preset defaults with overrides.
    ///
    /// This method applies the `LabelPreset` defaults first, then applies any user-specified
    /// overrides from optional fields. For example, if the preset is `Alpha` but `single_author_chars`
    /// is specified, the specified value takes precedence over the preset default of 3.
    ///
    /// # Returns
    ///
    /// A `LabelParams` struct with all parameters resolved to concrete values.
    pub fn effective_params(&self) -> LabelParams {
        let (
            default_single_author_chars,
            default_multi_author_chars,
            default_et_al_min,
            default_marker,
            default_et_al_names,
        ) = match self.preset {
            LabelPreset::Alpha => (3u8, 1u8, 4u8, "+".to_string(), 3u8),
            LabelPreset::Ams => (4u8, 1u8, 5u8, String::new(), 4u8),
            LabelPreset::Din => (4u8, 1u8, 3u8, String::new(), 3u8),
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
            et_al_names: self.et_al_names.unwrap_or(default_et_al_names),
            year_digits: self.year_digits.unwrap_or(2),
        }
    }
}

/// Processing mode for citation/bibliography generation.
///
/// Determines how citations and bibliographies are sorted, grouped, and disambiguated.
/// Can be specified as a simple string or with complex configuration maps:
/// - A string: `"author-date"`, `"author-date-full"`, `"numeric"`, `"note"`, or `"label"`
/// - A label config map: `{ label: { preset: din } }`
/// - A custom config map: `{ sort: ..., group: ..., disambiguate: ... }`
// `rename_all` is retained for `JsonSchema` derive (custom `Serialize` /
// `Deserialize` impls below already use kebab-case names directly).
#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum Processing {
    /// Author-date styles (e.g., APA, Chicago).
    /// Default bibliography ordering: author, year, title; disambiguates by year suffix.
    #[default]
    AuthorDate,
    /// Author-date styles that also add given names during disambiguation.
    AuthorDateGivenname,
    /// Author-date styles that also expand name lists during disambiguation.
    AuthorDateNames,
    /// Author-date styles that expand name lists and add given names during disambiguation.
    AuthorDateFull,
    /// Numeric styles (e.g., IEEE, Nature).
    /// Do not imply a bibliography sort; citations are numbered in order of appearance.
    Numeric,
    /// Note styles (e.g., Chicago Notes-Bibliography).
    /// With a bibliography default to author, title, year ordering.
    Note,
    /// Label styles (e.g., Alpha, DIN 1505-2).
    /// Default bibliography ordering: author, year, title.
    Label(LabelConfig),
    /// Fully custom processing behavior.
    /// Explicit `sort` configuration remains authoritative.
    Custom(ProcessingCustom),
}

/// How citation-item sorting is resolved when `citation.sort` is absent.
///
/// Determines whether citation clusters can be reordered automatically or only
/// when explicitly configured.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CitationSortPolicy {
    /// Only an explicit `citation.sort` can reorder multi-cite clusters.
    ExplicitOnly,
}

/// Named processing preset usable as the base of a custom delta.
///
/// Restricting `ProcessingCustom::base` to this enum makes nested custom
/// configurations impossible by construction: a base is always one of the
/// named presets, never another custom block.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ProcessingBase {
    /// Delta base equivalent to `Processing::AuthorDate`.
    AuthorDate,
    /// Delta base equivalent to `Processing::AuthorDateGivenname`.
    AuthorDateGivenname,
    /// Delta base equivalent to `Processing::AuthorDateNames`.
    AuthorDateNames,
    /// Delta base equivalent to `Processing::AuthorDateFull`.
    AuthorDateFull,
    /// Delta base equivalent to `Processing::Numeric`.
    Numeric,
    /// Delta base equivalent to `Processing::Note`.
    Note,
    /// Delta base equivalent to `Processing::Label` with default label config.
    Label,
}

impl ProcessingBase {
    /// Return whether this base belongs to the author-date family.
    #[must_use]
    pub fn supplies_author_substitution(self) -> bool {
        matches!(
            self,
            Self::AuthorDate
                | Self::AuthorDateGivenname
                | Self::AuthorDateNames
                | Self::AuthorDateFull
        )
    }

    /// The named `Processing` variant this base stands for.
    ///
    /// `Label` maps to `Processing::Label(LabelConfig::default())`; all other
    /// variants map to their unit counterparts.
    pub fn processing(&self) -> Processing {
        match self {
            Self::AuthorDate => Processing::AuthorDate,
            Self::AuthorDateGivenname => Processing::AuthorDateGivenname,
            Self::AuthorDateNames => Processing::AuthorDateNames,
            Self::AuthorDateFull => Processing::AuthorDateFull,
            Self::Numeric => Processing::Numeric,
            Self::Note => Processing::Note,
            Self::Label => Processing::Label(LabelConfig::default()),
        }
    }
}

/// Custom processing configuration.
///
/// Allows explicit specification of sorting, grouping, and disambiguation rules.
/// With a `base`, the block is a *delta*: present fields override the named
/// preset's configuration wholesale and absent fields inherit from it (the same
/// philosophy as style `extends:`). Without a `base`, present fields stand alone.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct ProcessingCustom {
    /// Named preset whose configuration seeds unset fields (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<ProcessingBase>,
    /// Bibliography sorting configuration (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortEntry>,
    /// Bibliography grouping configuration (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Group>,
    /// Disambiguation settings (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguate: Option<Disambiguation>,
}

impl ProcessingCustom {
    /// Resolve the effective configuration by overlaying present fields onto
    /// the base preset's config.
    ///
    /// Each present field (`sort`, `group`, `disambiguate`) replaces the base's
    /// value wholesale; absent fields inherit the base's. Without a `base`,
    /// returns the stored fields as-is. The result carries no `base` — it is
    /// fully materialized.
    #[must_use]
    pub fn resolved(&self) -> ProcessingCustom {
        let mut config = match self.base {
            Some(base) => base.processing().config(),
            None => ProcessingCustom::default(),
        };
        config.base = None;
        if self.sort.is_some() {
            config.sort = self.sort.clone();
        }
        if self.group.is_some() {
            config.group = self.group.clone();
        }
        if self.disambiguate.is_some() {
            config.disambiguate = self.disambiguate.clone();
        }
        config
    }
}

/// Coarse citation regime family for cross-regime compatibility checks.
///
/// Groups the `Processing` variants into mutually-exclusive citation-surface
/// families. Used by `merge_style_overlay` and `StyleLineage::apply_regime_guard`
/// to detect when a child's regime differs from its parent's, so that
/// regime-specific citation sub-specs (integral, non-integral) can be reset
/// rather than silently inherited.
///
/// See `docs/specs/CITATION_REGIME.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegimeFamily {
    /// All `AuthorDate*` variants: primary key is `(Author, Year)`.
    AuthorDate,
    /// `Numeric`: primary key is citation-order number.
    Numeric,
    /// `Note`: citations render as footnotes or endnotes.
    Note,
    /// `Label`: citations render as trigraph labels.
    Label,
    /// `Custom`: fully user-defined; never triggers automatic resets.
    Custom,
}

fn author_date_config(
    names: bool,
    add_givenname: bool,
    givenname_rule: GivennameRule,
) -> ProcessingCustom {
    ProcessingCustom {
        base: None,
        sort: Some(SortEntry::Preset(SortPreset::AuthorDateTitle)),
        group: Some(Group {
            template: vec![SortKey::Author, SortKey::Year],
        }),
        disambiguate: Some(Disambiguation {
            names,
            add_givenname,
            givenname_rule,
            year_suffix: true,
        }),
    }
}

impl Processing {
    /// Return whether this processing regime supplies the standard author chain.
    #[must_use]
    pub fn supplies_author_substitution(&self) -> bool {
        match self {
            Self::AuthorDate
            | Self::AuthorDateGivenname
            | Self::AuthorDateNames
            | Self::AuthorDateFull => true,
            Self::Custom(config) => config
                .base
                .is_some_and(ProcessingBase::supplies_author_substitution),
            Self::Numeric | Self::Note | Self::Label(_) => false,
        }
    }

    /// Default bibliography sort for the processing family, if any.
    ///
    /// Returns the standard bibliography sort order for the processing mode:
    /// - `AuthorDate` / `Label`: author, year, title
    /// - `Note`: author, title, year
    /// - `Numeric`: None (no automatic sort)
    /// - `Custom`: the base preset's default when a `base` is set and no
    ///   explicit `sort` overrides it; otherwise None (an explicit custom sort
    ///   resolves through `config()` instead, keeping the entry-ID tiebreak)
    pub fn default_bibliography_sort(&self) -> Option<SortPreset> {
        match self {
            Processing::AuthorDate
            | Processing::AuthorDateGivenname
            | Processing::AuthorDateNames
            | Processing::AuthorDateFull => Some(SortPreset::AuthorDateTitle),
            Processing::Numeric => None,
            Processing::Note => Some(SortPreset::AuthorTitleDate),
            Processing::Label(_) => Some(SortPreset::AuthorDateTitle),
            Processing::Custom(custom) => match (custom.base, custom.sort.as_ref()) {
                (Some(base), None) => base.processing().default_bibliography_sort(),
                _ => None,
            },
        }
    }

    /// Returns `true` for all author-date family variants.
    ///
    /// A `Custom` delta whose `base` is an author-date preset counts as
    /// author-date: a delta on author-date is still an author-date style.
    /// Centralizes the author-date family check so new variants don't require
    /// updating scattered `matches!` blocks across the codebase.
    pub fn is_author_date_family(&self) -> bool {
        self.regime_family() == RegimeFamily::AuthorDate
    }

    /// Coarse citation regime family for cross-regime compatibility checks.
    ///
    /// Used during style inheritance to determine whether an inherited parent's
    /// citation-mode sub-specs (integral, non-integral) belong to a different
    /// regime and should be reset when the child supplies its own base template.
    ///
    /// A base-less `Custom` is its own family and never triggers automatic
    /// sub-spec resets, preserving fully-custom authored styles. A `Custom`
    /// delta with a `base` belongs to its base's family: a delta on
    /// author-date is still an author-date style.
    ///
    /// See `docs/specs/CITATION_REGIME.md` for the full invariant.
    pub fn regime_family(&self) -> RegimeFamily {
        match self {
            Self::AuthorDate
            | Self::AuthorDateGivenname
            | Self::AuthorDateNames
            | Self::AuthorDateFull => RegimeFamily::AuthorDate,
            Self::Numeric => RegimeFamily::Numeric,
            Self::Note => RegimeFamily::Note,
            Self::Label(_) => RegimeFamily::Label,
            Self::Custom(custom) => match custom.base {
                Some(base) => base.processing().regime_family(),
                None => RegimeFamily::Custom,
            },
        }
    }

    /// Citation sorting remains explicit-only for all processing families.
    ///
    /// All processing modes use `ExplicitOnly`, meaning citation clusters are only
    /// reordered when explicitly configured via `citation.sort`.
    pub fn default_citation_sort_policy(&self) -> CitationSortPolicy {
        CitationSortPolicy::ExplicitOnly
    }

    /// Get the effective bibliography/disambiguation configuration for this processing mode.
    ///
    /// Returns a `ProcessingCustom` struct with the resolved configuration combining
    /// preset defaults and user overrides. For `Custom` mode, returns the user-provided config as-is.
    pub fn config(&self) -> ProcessingCustom {
        match self {
            Processing::AuthorDate => author_date_config(false, false, GivennameRule::ByCite),
            Processing::AuthorDateGivenname => {
                author_date_config(false, true, GivennameRule::ByCite)
            }
            Processing::AuthorDateNames => author_date_config(true, false, GivennameRule::ByCite),
            // `author-date-full` is the major author-date *guide* profile (APA §8.20,
            // Chicago AD): it adds names + given names + year suffix, and uses the
            // global `primary-name` rule so same-surname authors gain first-author
            // initials in *every* in-text cite. (Citum's `by-cite` default is
            // citation-local and would miss authors cited separately.) Initials vs full
            // form follow each style's `initialize-with`/`name-form` contributor config.
            Processing::AuthorDateFull => {
                author_date_config(true, true, GivennameRule::PrimaryName)
            }
            Processing::Numeric => ProcessingCustom::default(),
            Processing::Note => ProcessingCustom {
                base: None,
                sort: Some(SortEntry::Preset(SortPreset::AuthorTitleDate)),
                group: None,
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: false,
                }),
            },
            Processing::Label(_) => ProcessingCustom {
                base: None,
                sort: Some(SortEntry::Preset(SortPreset::AuthorDateTitle)),
                group: None,
                disambiguate: Some(Disambiguation {
                    names: false,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: true,
                }),
            },
            Processing::Custom(custom) => custom.resolved(),
        }
    }
}

impl Serialize for Processing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Processing::AuthorDate => serializer.serialize_str("author-date"),
            Processing::AuthorDateGivenname => serializer.serialize_str("author-date-givenname"),
            Processing::AuthorDateNames => serializer.serialize_str("author-date-names"),
            Processing::AuthorDateFull => serializer.serialize_str("author-date-full"),
            Processing::Numeric => serializer.serialize_str("numeric"),
            Processing::Note => serializer.serialize_str("note"),
            Processing::Label(config) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("label", config)?;
                map.end()
            }
            // Emit `Custom` as a bare map so the YAML reads
            // `processing:\n  sort: ...` instead of `processing: !custom`.
            // The `visit_map` deserializer above already accepts this shape.
            Processing::Custom(custom) => custom.serialize(serializer),
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
                    "author-date-givenname" => Ok(Processing::AuthorDateGivenname),
                    "author-date-names" => Ok(Processing::AuthorDateNames),
                    "author-date-full" => Ok(Processing::AuthorDateFull),
                    "numeric" => Ok(Processing::Numeric),
                    "note" => Ok(Processing::Note),
                    "label" => Ok(Processing::Label(LabelConfig::default())),
                    other => Err(E::unknown_variant(other, PROCESSING_STRING_VARIANTS)),
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
                    // `custom` is the only externally-tagged variant; named
                    // string forms are handled by `visit_str` above.
                    other => Err(de::Error::unknown_variant(other, &["custom"])),
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
                    "base" | "sort" | "group" | "disambiguate" => {
                        // This is a custom processing config
                        // We need to deserialize the whole map as ProcessingCustom
                        // Unfortunately we can't easily re-parse from the middle of map access.
                        // Instead, collect fields and build manually
                        let mut base = None;
                        let mut sort = None;
                        let mut group = None;
                        let mut disambiguate = None;

                        // Values deserialize as `Option<_>` so an explicit
                        // null (`base: ~`) reads as absent, matching derived
                        // serde semantics and the JSON schema contract.

                        // Handle the first key we already read
                        match key.as_str() {
                            "base" => base = map.next_value()?,
                            "sort" => sort = map.next_value()?,
                            "group" => group = map.next_value()?,
                            "disambiguate" => disambiguate = map.next_value()?,
                            _ => {
                                return Err(de::Error::unknown_field(
                                    &key,
                                    &["base", "sort", "group", "disambiguate"],
                                ));
                            }
                        }

                        // Read remaining keys
                        while let Some(k) = map.next_key::<String>()? {
                            match k.as_str() {
                                "base" => base = map.next_value()?,
                                "sort" => sort = map.next_value()?,
                                "group" => group = map.next_value()?,
                                "disambiguate" => disambiguate = map.next_value()?,
                                other => {
                                    return Err(de::Error::unknown_field(
                                        other,
                                        &["base", "sort", "group", "disambiguate"],
                                    ));
                                }
                            }
                        }

                        Ok(Processing::Custom(ProcessingCustom {
                            base,
                            sort,
                            group,
                            disambiguate,
                        }))
                    }
                    other => Err(de::Error::unknown_field(
                        other,
                        &["label", "base", "sort", "group", "disambiguate"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(ProcessingVisitor)
    }
}

/// Controls which author positions receive given-name expansion during disambiguation.
///
/// Maps to CSL's `givenname-disambiguation-rule` attribute on `<citation>`.
/// The engine collapses these to two scopes: `PrimaryName` and
/// `PrimaryNameWithInitials` expand only the first (primary) author; all other
/// values expand all positions. Initials vs full form is always driven by the
/// contributor config's `initialize-with` / `name-form` settings.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum GivennameRule {
    /// Disambiguate per-cite with a minimal subset of names (CSL 1.0.1 default).
    /// Engine behaviour: expand all positions (per-cite minimal-subset deferred).
    #[default]
    ByCite,
    /// Expand given names for all name positions.
    AllNames,
    /// Expand given names (initials form) for all name positions.
    AllNamesWithInitials,
    /// Expand given name of the first (primary) author only.
    PrimaryName,
    /// Expand given name (initials form) of the first (primary) author only.
    PrimaryNameWithInitials,
}

/// Disambiguation settings.
///
/// Controls how ambiguous citations are disambiguated in the output.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Disambiguation {
    /// Whether to attempt disambiguation by expanding author names.
    pub names: bool,
    /// Whether to add given names to disambiguate similarly-named authors.
    #[serde(default)]
    pub add_givenname: bool,
    /// Which author positions receive given-name expansion.
    #[serde(default)]
    pub givenname_rule: GivennameRule,
    /// Whether to append year suffixes (a, b, c, ...) for multiple works from the same author-year.
    pub year_suffix: bool,
}

impl Default for Disambiguation {
    fn default() -> Self {
        Self {
            names: true,
            add_givenname: false,
            givenname_rule: GivennameRule::default(),
            year_suffix: false,
        }
    }
}

/// Sorting configuration.
///
/// Specifies how bibliography entries are ordered.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Sort {
    /// Whether to shorten name lists for sorting the same as for display.
    #[serde(default)]
    pub shorten_names: bool,
    /// Whether to apply the same name substitutions during sorting as during rendering.
    #[serde(default)]
    pub render_substitutions: bool,
    /// Sort keys in order of application.
    pub template: Vec<SortSpec>,
}

/// Sort configuration: either a preset name or explicit configuration.
///
/// Can be a preset name like `author-date-title` or a full `Sort` struct with explicit settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum SortEntry {
    /// A named sort preset (e.g., `author-date-title`, `author-title-date`).
    Preset(crate::presets::SortPreset),
    /// Explicit sort configuration with custom keys and order.
    Explicit(Sort),
}

impl SortEntry {
    /// Resolve this entry to a concrete `Sort`.
    ///
    /// If this is a preset, returns the preset's sort definition. Otherwise returns the explicit sort as-is.
    pub fn resolve(&self) -> Sort {
        match self {
            SortEntry::Preset(preset) => preset.sort(),
            SortEntry::Explicit(sort) => sort.clone(),
        }
    }
}

impl Sort {
    /// Convert this config-level sort to a [`crate::grouping::GroupSort`].
    ///
    /// Keys with no group-sort equivalent are skipped rather than mapped:
    /// `CitationNumber` keeps registry order, since citation-number sorting
    /// is registry order by definition (see the engine's
    /// `citation_number_sort_not_supported` style-load warning). The match is
    /// deliberately exhaustive so adding a `SortKey` variant forces an
    /// explicit mapping decision here.
    pub fn group_sort(&self) -> crate::grouping::GroupSort {
        let template = self
            .template
            .iter()
            .filter_map(|sort| {
                let key = match sort.key {
                    SortKey::Author => crate::grouping::SortKey::Author,
                    SortKey::Year => crate::grouping::SortKey::Issued,
                    SortKey::Title => crate::grouping::SortKey::Title,
                    // No group-sort equivalent: citation-number sorting is
                    // registry order by definition (see the engine's
                    // `citation_number_sort_not_supported` warning).
                    SortKey::CitationNumber => return None,
                };
                Some(crate::grouping::GroupSortKey {
                    key,
                    ascending: sort.ascending,
                    order: None,
                    sort_order: None,
                })
            })
            .collect();

        crate::grouping::GroupSort { template }
    }
}

/// A single sort specification.
///
/// Defines one sort dimension with its key and direction.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct SortSpec {
    /// The field to sort by.
    pub key: SortKey,
    /// Whether to sort in ascending order (default: true).
    #[serde(default = "default_ascending")]
    pub ascending: bool,
}

fn default_ascending() -> bool {
    true
}

/// Available sort keys.
///
/// Specifies what field to sort bibliography entries by.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SortKey {
    /// Sort by the work's author(s).
    #[default]
    Author,
    /// Sort by publication year.
    Year,
    /// Sort by the work's title.
    Title,
    /// Sort by citation order (typically used for numeric styles).
    CitationNumber,
}

/// Grouping configuration for bibliography.
///
/// Specifies how bibliography entries should be grouped in the output.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Group {
    /// Sort keys used to define group boundaries (e.g., [Author, Year]).
    pub template: Vec<SortKey>,
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

    /// Test that LabelConfig::effective_params() applies Alpha preset defaults.
    #[test]
    fn test_label_config_alpha_preset_defaults() {
        let config = LabelConfig {
            preset: LabelPreset::Alpha,
            single_author_chars: None,
            multi_author_chars: None,
            et_al_min: None,
            et_al_marker: None,
            et_al_names: None,
            year_digits: None,
        };

        let params = config.effective_params();
        assert_eq!(params.single_author_chars, 3);
        assert_eq!(params.multi_author_chars, 1);
        assert_eq!(params.et_al_min, 4);
        assert_eq!(params.et_al_marker, "+");
        assert_eq!(params.et_al_names, 3);
        assert_eq!(params.year_digits, 2);
    }

    /// Test that LabelConfig overrides take precedence over preset defaults.
    #[test]
    fn test_label_config_alpha_with_overrides() {
        let config = LabelConfig {
            preset: LabelPreset::Alpha,
            single_author_chars: Some(5),
            multi_author_chars: Some(2),
            et_al_min: Some(5),
            et_al_marker: Some("*".to_string()),
            et_al_names: Some(4),
            year_digits: Some(4),
        };

        let params = config.effective_params();
        assert_eq!(params.single_author_chars, 5);
        assert_eq!(params.multi_author_chars, 2);
        assert_eq!(params.et_al_min, 5);
        assert_eq!(params.et_al_marker, "*");
        assert_eq!(params.et_al_names, 4);
        assert_eq!(params.year_digits, 4);
    }

    /// Test that LabelConfig::effective_params() applies Din preset defaults.
    #[test]
    fn test_label_config_din_preset_defaults() {
        let config = LabelConfig {
            preset: LabelPreset::Din,
            single_author_chars: None,
            multi_author_chars: None,
            et_al_min: None,
            et_al_marker: None,
            et_al_names: None,
            year_digits: None,
        };

        let params = config.effective_params();
        assert_eq!(params.single_author_chars, 4);
        assert_eq!(params.multi_author_chars, 1);
        assert_eq!(params.et_al_min, 3);
        assert_eq!(params.et_al_marker, "");
        assert_eq!(params.et_al_names, 3);
        assert_eq!(params.year_digits, 2);
    }

    /// Test that LabelConfig::effective_params() applies AMS/CSL label defaults.
    #[test]
    fn test_label_config_ams_preset_defaults() {
        let config = LabelConfig {
            preset: LabelPreset::Ams,
            single_author_chars: None,
            multi_author_chars: None,
            et_al_min: None,
            et_al_marker: None,
            et_al_names: None,
            year_digits: None,
        };

        let params = config.effective_params();
        assert_eq!(params.single_author_chars, 4);
        assert_eq!(params.multi_author_chars, 1);
        assert_eq!(params.et_al_min, 5);
        assert_eq!(params.et_al_marker, "");
        assert_eq!(params.et_al_names, 4);
        assert_eq!(params.year_digits, 2);
    }

    /// Test that Processing::AuthorDate returns correct default sort.
    #[test]
    fn test_processing_author_date_default_bibliography_sort() {
        let processing = Processing::AuthorDate;
        let sort = processing.default_bibliography_sort();
        assert_eq!(sort, Some(SortPreset::AuthorDateTitle));
    }

    /// Test that Processing::Numeric returns no default sort.
    #[test]
    fn test_processing_numeric_default_bibliography_sort() {
        let processing = Processing::Numeric;
        let sort = processing.default_bibliography_sort();
        assert_eq!(sort, None);
    }

    /// Test that Processing::Note returns correct default sort.
    #[test]
    fn test_processing_note_default_bibliography_sort() {
        let processing = Processing::Note;
        let sort = processing.default_bibliography_sort();
        assert_eq!(sort, Some(SortPreset::AuthorTitleDate));
    }

    /// Test that all Processing modes return ExplicitOnly citation sort policy.
    #[test]
    fn test_processing_citation_sort_policy() {
        let modes = vec![
            Processing::AuthorDate,
            Processing::AuthorDateGivenname,
            Processing::AuthorDateNames,
            Processing::AuthorDateFull,
            Processing::Numeric,
            Processing::Note,
            Processing::Label(LabelConfig::default()),
            Processing::Custom(ProcessingCustom::default()),
        ];

        for mode in modes {
            assert_eq!(
                mode.default_citation_sort_policy(),
                CitationSortPolicy::ExplicitOnly
            );
        }
    }

    /// Test that Processing::config() returns correct configuration for author-date variants.
    #[test]
    fn test_processing_author_date_variant_configs() {
        let cases = [
            (Processing::AuthorDate, false, false, GivennameRule::ByCite),
            (
                Processing::AuthorDateGivenname,
                false,
                true,
                GivennameRule::ByCite,
            ),
            (
                Processing::AuthorDateNames,
                true,
                false,
                GivennameRule::ByCite,
            ),
            // Only `author-date-full` (the guide profile) uses the global primary-name rule.
            (
                Processing::AuthorDateFull,
                true,
                true,
                GivennameRule::PrimaryName,
            ),
        ];

        for (processing, names, add_givenname, expected_rule) in cases {
            let config = processing.config();

            assert_eq!(
                config.sort,
                Some(SortEntry::Preset(SortPreset::AuthorDateTitle))
            );
            assert_eq!(
                config.group,
                Some(Group {
                    template: vec![SortKey::Author, SortKey::Year],
                })
            );

            let disambig = config.disambiguate.unwrap();
            assert_eq!(disambig.names, names);
            assert_eq!(disambig.add_givenname, add_givenname);
            assert_eq!(disambig.givenname_rule, expected_rule);
            assert!(disambig.year_suffix);
        }
    }

    /// Test that author-date processing variants round-trip through their public names.
    #[test]
    fn test_processing_author_date_variant_names() {
        let cases = [
            (Processing::AuthorDate, "author-date"),
            (Processing::AuthorDateGivenname, "author-date-givenname"),
            (Processing::AuthorDateNames, "author-date-names"),
            (Processing::AuthorDateFull, "author-date-full"),
        ];

        for (processing, name) in cases {
            let serialized = serde_yaml::to_string(&processing).unwrap();
            assert_eq!(serialized.trim(), name);

            let deserialized: Processing = serde_yaml::from_str(name).unwrap();
            assert_eq!(deserialized, processing);
        }
    }

    /// Test that a custom map with `base:` and an explicit sort round-trips
    /// through YAML preserving the sparse delta shape.
    #[test]
    fn test_processing_custom_base_round_trip() {
        // given: a custom delta on author-date with only an explicit sort
        let processing = Processing::Custom(ProcessingCustom {
            base: Some(ProcessingBase::AuthorDate),
            sort: Some(SortEntry::Preset(SortPreset::AuthorTitleDate)),
            group: None,
            disambiguate: None,
        });

        // when: serialized to YAML and parsed back
        let yaml = serde_yaml::to_string(&processing).unwrap();
        let parsed: Processing = serde_yaml::from_str(&yaml).unwrap();

        // then: the YAML stays sparse (base + sort only) and round-trips
        assert_eq!(yaml.trim(), "base: author-date\nsort: author-title-date");
        assert_eq!(parsed, processing);
    }

    /// Test that a map with only `base:` parses and resolves to the bare
    /// preset's config.
    #[test]
    fn test_processing_custom_base_only_resolves_to_preset_config() {
        // given: YAML declaring only a base
        let parsed: Processing = serde_yaml::from_str("base: author-date-full").unwrap();

        // then: it parses as Custom with the base and no overrides
        assert_eq!(
            parsed,
            Processing::Custom(ProcessingCustom {
                base: Some(ProcessingBase::AuthorDateFull),
                sort: None,
                group: None,
                disambiguate: None,
            })
        );

        // and: config() matches the bare preset's config
        assert_eq!(parsed.config(), Processing::AuthorDateFull.config());
    }

    /// Test that resolved() overlays present fields wholesale and inherits
    /// absent fields from the base preset.
    #[test]
    fn test_processing_custom_resolved_overlay_semantics() {
        // given: a delta on author-date overriding only the sort
        let custom = ProcessingCustom {
            base: Some(ProcessingBase::AuthorDate),
            sort: Some(SortEntry::Preset(SortPreset::AuthorTitleDate)),
            group: None,
            disambiguate: None,
        };

        // when: resolved against the base
        let resolved = custom.resolved();

        // then: the explicit sort wins; group/disambiguate inherit; no base remains
        let base_config = Processing::AuthorDate.config();
        assert_eq!(resolved.base, None);
        assert_eq!(
            resolved.sort,
            Some(SortEntry::Preset(SortPreset::AuthorTitleDate))
        );
        assert_eq!(resolved.group, base_config.group);
        assert_eq!(resolved.disambiguate, base_config.disambiguate);
    }

    /// Test that resolved() without a base returns the stored fields as-is.
    #[test]
    fn test_processing_custom_resolved_without_base_is_identity() {
        let custom = ProcessingCustom {
            base: None,
            sort: Some(SortEntry::Preset(SortPreset::AuthorDateTitle)),
            group: None,
            disambiguate: None,
        };

        assert_eq!(custom.resolved(), custom);
    }

    /// Test that regime_family and is_author_date_family delegate to the base
    /// when present and stay Custom without one.
    #[test]
    fn test_processing_custom_base_family_delegation() {
        // given: a delta on author-date and a base-less custom
        let with_base = Processing::Custom(ProcessingCustom {
            base: Some(ProcessingBase::AuthorDate),
            ..Default::default()
        });
        let without_base = Processing::Custom(ProcessingCustom::default());

        // then: family checks follow the base only when one is set
        assert_eq!(with_base.regime_family(), RegimeFamily::AuthorDate);
        assert!(with_base.is_author_date_family());
        assert_eq!(without_base.regime_family(), RegimeFamily::Custom);
        assert!(!without_base.is_author_date_family());

        // and: a numeric base maps to the numeric family
        let numeric_base = Processing::Custom(ProcessingCustom {
            base: Some(ProcessingBase::Numeric),
            ..Default::default()
        });
        assert_eq!(numeric_base.regime_family(), RegimeFamily::Numeric);
        assert!(!numeric_base.is_author_date_family());
    }

    /// Test that default_bibliography_sort delegates to the base only when the
    /// custom carries no explicit sort.
    #[test]
    fn test_processing_custom_base_default_bibliography_sort() {
        // given: a base-carrying custom without an explicit sort
        let inherited = Processing::Custom(ProcessingCustom {
            base: Some(ProcessingBase::AuthorDate),
            ..Default::default()
        });
        // then: the base's preset default applies
        assert_eq!(
            inherited.default_bibliography_sort(),
            Some(SortPreset::AuthorDateTitle)
        );

        // given: the same base with an explicit sort override
        let overridden = Processing::Custom(ProcessingCustom {
            base: Some(ProcessingBase::AuthorDate),
            sort: Some(SortEntry::Preset(SortPreset::AuthorTitleDate)),
            ..Default::default()
        });
        // then: no preset default — the explicit sort resolves via config()
        assert_eq!(overridden.default_bibliography_sort(), None);
    }

    /// Test that explicit null values in a custom map read as absent fields,
    /// matching derived-serde `Option` semantics and the JSON schema.
    #[test]
    fn test_processing_custom_map_accepts_explicit_nulls() {
        // given: a custom map with explicit nulls alongside a real field
        let parsed: Processing =
            serde_yaml::from_str("base: ~\nsort: author-title-date\ndisambiguate: null").unwrap();

        // then: null fields are absent, the real field is kept
        assert_eq!(
            parsed,
            Processing::Custom(ProcessingCustom {
                base: None,
                sort: Some(SortEntry::Preset(SortPreset::AuthorTitleDate)),
                group: None,
                disambiguate: None,
            })
        );
    }

    /// Test that invalid `base:` values are rejected at parse time.
    #[test]
    fn test_processing_custom_base_rejects_invalid_values() {
        // given: a nested map and an unknown preset name as base
        let nested = serde_yaml::from_str::<Processing>("base: { sort: author-date-title }");
        let unknown = serde_yaml::from_str::<Processing>("base: fancy-date");

        // then: both fail to parse
        assert!(nested.is_err());
        assert!(unknown.is_err());
    }

    /// Test that Disambiguation defaults have correct values.
    #[test]
    fn test_disambiguation_defaults() {
        let disambig = Disambiguation::default();
        assert!(disambig.names);
        assert!(!disambig.add_givenname);
        assert_eq!(disambig.givenname_rule, GivennameRule::ByCite);
        assert!(!disambig.year_suffix);
    }

    /// Test that SortEntry::resolve() returns preset sort for Preset variant.
    #[test]
    fn test_sort_entry_resolve_preset() {
        let entry = SortEntry::Preset(SortPreset::AuthorDateTitle);
        let sort = entry.resolve();

        // Verify it resolves to a valid Sort
        assert!(!sort.template.is_empty());
    }

    /// Test that `Sort::group_sort()` maps author/year/title keys and skips
    /// `CitationNumber` (which has no group-sort equivalent).
    #[test]
    fn test_sort_group_sort_maps_keys_and_skips_citation_number() {
        let sort = Sort {
            shorten_names: false,
            render_substitutions: false,
            template: vec![
                SortSpec {
                    key: SortKey::Author,
                    ascending: true,
                },
                SortSpec {
                    key: SortKey::Year,
                    ascending: false,
                },
                SortSpec {
                    key: SortKey::Title,
                    ascending: true,
                },
                SortSpec {
                    key: SortKey::CitationNumber,
                    ascending: true,
                },
            ],
        };

        let group_sort = sort.group_sort();

        assert_eq!(group_sort.template.len(), 3);
        assert_eq!(group_sort.template[0].key, crate::grouping::SortKey::Author);
        assert!(group_sort.template[0].ascending);
        assert_eq!(group_sort.template[1].key, crate::grouping::SortKey::Issued);
        assert!(!group_sort.template[1].ascending);
        assert_eq!(group_sort.template[2].key, crate::grouping::SortKey::Title);
        assert!(group_sort.template[2].ascending);
    }

    /// Test that SortEntry::resolve() returns explicit sort for Explicit variant.
    #[test]
    fn test_sort_entry_resolve_explicit() {
        let explicit = Sort {
            shorten_names: true,
            render_substitutions: false,
            template: vec![SortSpec {
                key: SortKey::Title,
                ascending: false,
            }],
        };
        let entry = SortEntry::Explicit(explicit.clone());
        let resolved = entry.resolve();

        assert!(resolved.shorten_names);
        assert!(!resolved.render_substitutions);
        assert_eq!(resolved.template.len(), 1);
        assert_eq!(resolved.template[0].key, SortKey::Title);
        assert!(!resolved.template[0].ascending);
    }
}
