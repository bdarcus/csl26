/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Locale-specific term types and definitions.
//!
//! This module defines the data structures for representing locale information including
//! general terms (prepositions, conjunctions), contributor role terms, locator terms for
//! pages and chapters, date-related terms, and month names. These are used by citation
//! processors to render localized output.

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

crate::str_enum! {
    /// Form for term lookup.
    ///
    /// Specifies which form variant of a term should be used in citation output.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum TermForm {
        /// Long form of a term (e.g., "page" vs "p.").
        Long = "long",
        /// Short form of a term (e.g., "p." vs "page").
        Short = "short",
        /// Verb form of a term (e.g., "edited by").
        Verb = "verb",
        /// Short verb form of a term (e.g., "ed." vs "edited by").
        VerbShort = "verb-short",
        /// Symbol form of a term (e.g., "§" for section).
        Symbol = "symbol"
    }
}

crate::str_enum! {
    /// Grammatical gender used for locale agreement and term selection.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum GrammaticalGender {
        /// Masculine grammatical gender.
        Masculine = "masculine",
        /// Feminine grammatical gender.
        Feminine = "feminine",
        /// Neuter grammatical gender.
        Neuter = "neuter",
        /// Common or shared grammatical gender.
        Common = "common"
    }
}

/// A value that may vary by grammatical gender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum MaybeGendered<T> {
    /// The value is the same for all genders.
    Plain(T),
    /// The value varies by grammatical gender.
    Gendered {
        /// Masculine variant.
        #[serde(skip_serializing_if = "Option::is_none")]
        masculine: Option<T>,
        /// Feminine variant.
        #[serde(skip_serializing_if = "Option::is_none")]
        feminine: Option<T>,
        /// Neuter variant.
        #[serde(skip_serializing_if = "Option::is_none")]
        neuter: Option<T>,
        /// Common or gender-unspecified variant.
        #[serde(skip_serializing_if = "Option::is_none")]
        common: Option<T>,
    },
}

impl<T: Default> Default for MaybeGendered<T> {
    fn default() -> Self {
        Self::Plain(T::default())
    }
}

impl<T> From<T> for MaybeGendered<T> {
    fn from(value: T) -> Self {
        Self::Plain(value)
    }
}

impl From<&str> for MaybeGendered<String> {
    fn from(value: &str) -> Self {
        Self::Plain(value.to_string())
    }
}

impl<T> MaybeGendered<T> {
    fn by_gender(&self, requested: GrammaticalGender) -> Option<&T> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            } => match requested {
                GrammaticalGender::Masculine => masculine.as_ref(),
                GrammaticalGender::Feminine => feminine.as_ref(),
                GrammaticalGender::Neuter => neuter.as_ref(),
                GrammaticalGender::Common => common.as_ref(),
                _ => None,
            },
        }
    }

    /// Resolve only the explicitly requested slot.
    pub fn resolve_strict(&self, requested: Option<GrammaticalGender>) -> Option<&T> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Gendered { .. } => requested.and_then(|gender| self.by_gender(gender)),
        }
    }

    /// Resolve a value using the documented production fallback order.
    pub fn resolve_with_fallback(&self, requested: Option<GrammaticalGender>) -> Option<&T> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            } => requested
                .and_then(|gender| self.by_gender(gender))
                .or(common.as_ref())
                .or(masculine.as_ref())
                .or(feminine.as_ref())
                .or(neuter.as_ref()),
        }
    }

    /// Resolve to a neutral/default form without selecting gendered slots.
    pub fn resolve_neutral(&self) -> Option<&T> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Gendered { common, .. } => common.as_ref(),
        }
    }
}

impl MaybeGendered<String> {
    /// Resolve to the default production string.
    pub fn as_default_str(&self) -> &str {
        self.resolve_with_fallback(None)
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Whether the default resolved value is empty.
    pub fn is_empty(&self) -> bool {
        self.as_default_str().is_empty()
    }

    /// Resolve to a borrowed string using the default production path.
    pub fn as_str(&self) -> &str {
        self.as_default_str()
    }

    /// Lowercase the default resolved value.
    pub fn to_lowercase(&self) -> String {
        self.as_default_str().to_lowercase()
    }
}

crate::str_enum! {
    /// A list of general terms for citation formatting.
    ///
    /// These are the standard terms that appear in bibliographies and citations,
    /// including prepositions (in, at, from, by), punctuation terms (and, et al),
    /// date-related terms (accessed, no-date, circa), locator terms (page, chapter, volume),
    /// and special phrases (ibid, forthcoming, available-at).
    #[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
    pub enum GeneralTerm {
        #[default]
        /// The preposition "in" (e.g., "in Smith, 2020").
        In = "in",
        /// The term used for access dates (e.g., "accessed May 1").
        Accessed = "accessed",
        /// The term used to introduce citation access dates (e.g., "cited May 1").
        Cited = "cited",
        /// The term used for retrieval statements (e.g., "retrieved from URL").
        Retrieved = "retrieved",
        /// The preposition "at" (e.g., "at the conference").
        At = "at",
        /// The preposition "from" (e.g., "from the publisher").
        From = "from",
        /// The preposition "of" (e.g., "special issue of").
        Of = "of",
        /// The preposition "to" (e.g., "from x to y").
        To = "to",
        /// The preposition "by" (e.g., "by John Smith").
        By = "by",
        /// The term used when no date is available (e.g., "n.d.").
        NoDate = "no-date",
        /// The term used for anonymous authorship (e.g., "anonymous").
        Anonymous = "anonymous",
        /// The term used for approximate dates (e.g., "circa").
        Circa = "circa",
        /// The phrase used for availability statements (e.g., "available at URL").
        AvailableAt = "available-at",
        /// The term used for immediately repeated citations (e.g., "ibid.").
        Ibid = "ibid",
        /// The conjunction "and" (e.g., "Smith and Jones").
        And = "and",
        /// Verbatim connector used between terms in a combined contributor role.
        RoleConjunction = "role-conjunction",
        /// The abbreviation for omitted additional names (e.g., "et al.").
        EtAl = "et-al",
        /// The phrase "and others" (generic use).
        AndOthers = "and-others",
        /// The term used for forthcoming works (e.g., "forthcoming").
        Forthcoming = "forthcoming",
        /// The term used for online resources (e.g., "online").
        Online = "online",
        /// The adverb "here".
        Here = "here",
        /// The term used for deposited materials.
        Deposited = "deposited",
        /// The phrase used to introduce reviewed works (e.g., "review of").
        ReviewOf = "review-of",
        /// The phrase used for original publication references (e.g., "originally published").
        OriginalWorkPublished = "original-work-published",
        /// The term used for patents (e.g., "patent").
        Patent = "patent",
        /// The term used for "issued" in patent entries (e.g., ", issued June 9, 2010").
        Issued = "issued",
        /// The general term for volume locators (e.g., "volume", "vol.").
        Volume = "volume",
        /// The general term for issue locators (e.g., "issue", "no.").
        Issue = "issue",
        /// The general term for page locators (e.g., "page", "p.", "pp.").
        Page = "page",
        /// The general term for chapter locators (e.g., "chapter", "ch.").
        Chapter = "chapter",
        /// The general term for editions (e.g., "edition", "ed.").
        Edition = "edition",
        /// The general term for section locators (e.g., "section", "§").
        Section = "section",
        /// The label for personal communications (e.g., "personal communication").
        PersonalCommunication = "personal-communication",
        /// The general term for a version/release label (e.g., "version" in "Version 2.1").
        Version = "version"
    }
}

/// General terms used in citations and bibliographies.
///
/// Contains prepositions, conjunctions, and common phrases that appear in citation output.
/// Includes both simple string terms and `SimpleTerm` fields with long/short variants.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Terms {
    /// The word "and" (e.g., "Smith and Jones").
    pub and: Option<String>,
    /// Symbol form of "and" (e.g., "&").
    pub and_symbol: Option<String>,
    /// "and others" phrase for generic use.
    pub and_others: Option<String>,
    /// Anonymous author term (has long and short forms).
    #[serde(default)]
    pub anonymous: SimpleTerm,
    /// "at" preposition.
    pub at: Option<String>,
    /// "accessed" term for URLs.
    pub accessed: Option<String>,
    /// "available at" phrase for URLs.
    pub available_at: Option<String>,
    /// "by" preposition.
    pub by: Option<String>,
    /// "circa" term for approximate dates (has long and short forms).
    #[serde(default)]
    pub circa: SimpleTerm,
    /// "et al." abbreviation.
    pub et_al: Option<String>,
    /// "from" preposition.
    pub from: Option<String>,
    /// "ibid." term for repeated citations.
    pub ibid: Option<String>,
    /// "in" preposition.
    pub in_: Option<String>,
    /// Legacy short-form fallback for the "no date" term when no structured term is loaded.
    ///
    /// This remains deserializable for backward compatibility, but it is not serialized
    /// to avoid colliding with the structured `no-date` entry from `general`.
    #[serde(skip_serializing)]
    pub no_date: Option<String>,
    /// "retrieved" term for access dates.
    pub retrieved: Option<String>,
    /// All other general terms flattened into a map.
    #[serde(flatten, default)]
    pub general: std::collections::HashMap<GeneralTerm, SimpleTerm>,
}

/// A simple term with long and short forms.
///
/// Used for terms that have a primary long form and a shorter variant.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SimpleTerm {
    /// The long form of the term (e.g., "anonymous").
    pub long: MaybeGendered<String>,
    /// The short form of the term (e.g., "anon.").
    pub short: MaybeGendered<String>,
}

/// Terms for contributor roles.
///
/// Defines forms for roles like editor, translator, etc. in singular, plural, and verb forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ContributorTerm {
    /// Singular form (e.g., "editor", "translator").
    pub singular: SimpleTerm,
    /// Plural form (e.g., "editors", "translators").
    pub plural: SimpleTerm,
    /// Verb form (e.g., "edited by", "translated by").
    pub verb: SimpleTerm,
}

/// Terms for locators (page, chapter, etc.).
///
/// Defines forms for locator terms that can appear in long, short, and symbol variants,
/// each with singular and plural options.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct LocatorTerm {
    /// Long form (e.g., "page"/"pages").
    #[serde(default)]
    pub long: Option<SingularPlural>,
    /// Short form (e.g., "p."/"pp.").
    #[serde(default)]
    pub short: Option<SingularPlural>,
    /// Symbol form (e.g., "§"/"§§").
    #[serde(default)]
    pub symbol: Option<SingularPlural>,
    /// Lexical gender for noun agreement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<GrammaticalGender>,
}

/// A term with singular and plural forms.
///
/// Used to represent terms that change depending on count, such as "page" vs "pages".
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SingularPlural {
    /// Singular form (e.g., "page").
    pub singular: MaybeGendered<String>,
    /// Plural form (e.g., "pages").
    pub plural: MaybeGendered<String>,
}

/// Date-related terms.
///
/// Contains month names, season names, and terms for date modifiers like uncertainty,
/// open-ended ranges, and time period notation.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct DateTerms {
    /// Month names (full and abbreviated forms).
    #[serde(default)]
    pub months: MonthNames,
    /// Season names, keyed by EDTF season code (`21`=Spring..`24`=Winter).
    #[serde(default)]
    pub seasons: BTreeMap<SubYearCode, String>,
    /// Term for uncertain dates (e.g., "uncertain").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncertainty_term: Option<String>,
    /// Term for open-ended date ranges (e.g., "present").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_ended_term: Option<String>,
    /// AM period term (e.g., "AM").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub am: Option<String>,
    /// PM period term (e.g., "PM").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm: Option<String>,
    /// UTC timezone term (e.g., "UTC").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone_utc: Option<String>,
    /// Era suffix for year zero and negative years (e.g., "BC").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_era: Option<String>,
    /// Era suffix for positive years in BC/AD profile (e.g., "AD").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad: Option<String>,
    /// Era suffix for negative years in BC/AD profile (e.g., "BC").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bc: Option<String>,
    /// Era suffix for negative years in BCE/CE profile (e.g., "BCE").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bce: Option<String>,
    /// Era suffix for positive years in BCE/CE profile (e.g., "CE").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ce: Option<String>,
}

/// Month name lists.
///
/// Contains both full and abbreviated month names for a given locale, each
/// keyed by EDTF sub-year code (`1`-`12`). Also reused, sparsely populated,
/// by [`LocaleOverride::dates`]'s month-name overrides — see
/// `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MonthNames {
    /// Full month names (e.g., "January", "February", ..., "December").
    pub long: BTreeMap<SubYearCode, String>,
    /// Abbreviated month names (e.g., "Jan.", "Feb.", ..., "Dec.").
    pub short: BTreeMap<SubYearCode, String>,
}

/// An EDTF sub-year code: a value more specific than a year but not
/// necessarily a calendar month.
///
/// `1`-`12` are calendar months (EDTF Level 0). `21`-`24` are the EDTF
/// Level 1 seasons (Spring, Summer, Autumn, Winter). `25`-`41` are reserved
/// for EDTF Level 2 sub-year granularity (quarters, semesters,
/// quadrimesters, hemisphere-qualified seasons) not yet modeled here — the
/// range is reserved so a future extension does not need another key shape.
///
/// Used as the map key for locale month/season name tables
/// ([`MonthNames`], [`DateTerms::seasons`]) and their sparse overrides, so a
/// style can replace a single month's name without redeclaring the other
/// eleven. See `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubYearCode(u8);

impl SubYearCode {
    /// First calendar-month code.
    pub const MIN_MONTH: u8 = 1;
    /// Last calendar-month code.
    pub const MAX_MONTH: u8 = 12;
    /// First EDTF Level 1 season code (Spring).
    pub const MIN_SEASON: u8 = 21;
    /// Last code reserved for EDTF Level 2 sub-year granularity.
    pub const MAX_SEASON: u8 = 41;

    /// Construct a `SubYearCode`, returning `None` if `code` falls outside
    /// the reserved month (`1`-`12`) or season/sub-year (`21`-`41`) ranges.
    #[must_use]
    pub fn new(code: u8) -> Option<Self> {
        let in_month_range = (Self::MIN_MONTH..=Self::MAX_MONTH).contains(&code);
        let in_season_range = (Self::MIN_SEASON..=Self::MAX_SEASON).contains(&code);
        (in_month_range || in_season_range).then_some(Self(code))
    }

    /// The raw EDTF sub-year code.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for SubYearCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SubYearCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for SubYearCode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SubYearCodeVisitor;

        impl serde::de::Visitor<'_> for SubYearCodeVisitor {
            type Value = SubYearCode;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "an EDTF sub-year code (1-12 for months, 21-41 reserved for seasons)"
                )
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                let code = u8::try_from(value)
                    .map_err(|_| E::custom(format!("sub-year code out of range: {value}")))?;
                SubYearCode::new(code)
                    .ok_or_else(|| E::custom(format!("sub-year code out of range: {code}")))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                let code: u8 = value
                    .parse()
                    .map_err(|_| E::custom(format!("invalid sub-year code: {value}")))?;
                SubYearCode::new(code)
                    .ok_or_else(|| E::custom(format!("sub-year code out of range: {code}")))
            }
        }

        // `deserialize_any` (not `deserialize_u8`) so the visitor's
        // `visit_u64`/`visit_str` are reached however the format represents
        // the value. Some deserializers only honor a numeric type hint by
        // special-casing it in map-key position; `deserialize_any` doesn't
        // depend on that and works for a plain field too.
        deserializer.deserialize_any(SubYearCodeVisitor)
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for SubYearCode {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("SubYearCode")
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "description": "EDTF sub-year code: 1-12 = calendar month, 21-24 = EDTF Level 1 season, 25-41 reserved for future sub-year codes.",
            "anyOf": [
                { "type": "integer", "minimum": 1, "maximum": u32::from(SubYearCode::MAX_MONTH) },
                {
                    "type": "integer",
                    "minimum": u32::from(SubYearCode::MIN_SEASON),
                    "maximum": u32::from(SubYearCode::MAX_SEASON),
                },
            ],
        })
    }
}

/// Number formatting options for a locale.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct NumberFormats {
    /// Digit glyph system used when rendering number template components.
    #[serde(default)]
    pub digit_system: DigitSystem,
    /// Decimal separator (e.g., "." for en-US, "," for de-DE).
    #[serde(default = "default_decimal_separator")]
    pub decimal_separator: String,
    /// Thousands separator (e.g., "," for en-US, "." for de-DE).
    #[serde(default = "default_thousands_separator")]
    pub thousands_separator: String,
    /// Minimum digits to display.
    #[serde(default = "default_minimum_digits")]
    pub minimum_digits: u8,
}

crate::str_enum! {
    /// Digit glyph system used by a locale when rendering numeric values.
    #[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
    pub enum DigitSystem {
        /// ASCII Western digits (`0` through `9`).
        #[default]
        Western = "western",
        /// Arabic-Indic digits (`٠` through `٩`).
        ArabicIndic = "arabic-indic",
        /// Extended Arabic-Indic digits (`۰` through `۹`).
        ExtendedArabicIndic = "extended-arabic-indic",
        /// Devanagari digits (`०` through `९`).
        Devanagari = "devanagari"
    }
}

fn default_decimal_separator() -> String {
    ".".into()
}

fn default_thousands_separator() -> String {
    ",".into()
}

fn default_minimum_digits() -> u8 {
    1
}

/// Grammar options that vary by language or regional convention.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct GrammarOptions {
    /// Whether to place periods/commas inside closing quotation marks (American style).
    #[serde(default)]
    pub punctuation_in_quote: bool,
    /// Whether to use a non-breaking space before colon/question mark (French style).
    #[serde(default)]
    pub nbsp_before_colon: bool,
    /// Opening outer quotation mark character.
    #[serde(default = "default_open_quote")]
    pub open_quote: String,
    /// Closing outer quotation mark character.
    #[serde(default = "default_close_quote")]
    pub close_quote: String,
    /// Opening inner (nested) quotation mark character.
    #[serde(default = "default_open_inner_quote")]
    pub open_inner_quote: String,
    /// Closing inner (nested) quotation mark character.
    #[serde(default = "default_close_inner_quote")]
    pub close_inner_quote: String,
    /// Whether to use a serial (Oxford) comma before the final list item.
    #[serde(default)]
    pub serial_comma: bool,
    /// Delimiter between page range endpoints.
    #[serde(default = "default_page_range_delimiter")]
    pub page_range_delimiter: String,
    /// Delimiter between a structured title's main title and subtitle group.
    #[serde(default = "default_title_subtitle_delimiter")]
    pub title_subtitle_delimiter: String,
    /// Delimiter between subtitle parts inside a structured title.
    #[serde(default = "default_subtitle_delimiter")]
    pub subtitle_delimiter: String,
    /// Policy for a strong terminal mark followed by a style-supplied comma.
    #[serde(default)]
    pub strong_terminal_comma_policy: crate::options::StrongTerminalCommaPolicy,
    /// Terminal marks that suppress a following delimiter's punctuation core.
    #[serde(default = "default_delimiter_suppressing_terminal_marks")]
    pub delimiter_suppressing_terminal_marks: String,
    /// Default placement of movable punctuation relative to closing
    /// quotation marks when a footnote marker is introduced. Overridable
    /// per-style via `options.notes.punctuation`.
    #[serde(default)]
    pub note_punctuation: crate::options::NoteQuotePlacement,
    /// Default placement of the footnote number marker relative to closing
    /// quotation marks. Overridable per-style via `options.notes.number`.
    #[serde(default)]
    pub note_number: crate::options::NoteNumberPlacement,
    /// Default order of the footnote marker relative to adjacent movable
    /// punctuation. Overridable per-style via `options.notes.order`.
    #[serde(default)]
    pub note_marker_order: crate::options::NoteMarkerOrder,
}

impl Default for GrammarOptions {
    fn default() -> Self {
        Self {
            punctuation_in_quote: false,
            nbsp_before_colon: false,
            open_quote: default_open_quote(),
            close_quote: default_close_quote(),
            open_inner_quote: default_open_inner_quote(),
            close_inner_quote: default_close_inner_quote(),
            serial_comma: false,
            page_range_delimiter: default_page_range_delimiter(),
            title_subtitle_delimiter: default_title_subtitle_delimiter(),
            subtitle_delimiter: default_subtitle_delimiter(),
            strong_terminal_comma_policy: crate::options::StrongTerminalCommaPolicy::default(),
            delimiter_suppressing_terminal_marks: default_delimiter_suppressing_terminal_marks(),
            note_punctuation: crate::options::NoteQuotePlacement::default(),
            note_number: crate::options::NoteNumberPlacement::default(),
            note_marker_order: crate::options::NoteMarkerOrder::default(),
        }
    }
}

fn default_open_quote() -> String {
    "\u{201C}".into()
}

fn default_close_quote() -> String {
    "\u{201D}".into()
}

fn default_open_inner_quote() -> String {
    "\u{2018}".into()
}

fn default_close_inner_quote() -> String {
    "\u{2019}".into()
}

fn default_page_range_delimiter() -> String {
    "\u{2013}".into()
}

fn default_title_subtitle_delimiter() -> String {
    ": ".into()
}

fn default_subtitle_delimiter() -> String {
    "; ".into()
}

fn default_delimiter_suppressing_terminal_marks() -> String {
    "?!…".into()
}

/// Message syntax variant active in a locale file.
///
/// Controls which `MessageEvaluator` implementation the engine selects at
/// locale-load time.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MessageSyntax {
    /// Plain text only; parameterized messages are not evaluated.
    #[default]
    Static,
    /// ICU Message Format 2 evaluation (requires `Mf2MessageEvaluator`).
    Mf2,
}

/// Runtime evaluation options for a locale.
///
/// Declares which message syntax the `messages` map uses and controls
/// evaluator selection. May grow with additional fields (custom function
/// declarations, evaluator hints) without breaking existing locale files.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct EvaluationConfig {
    /// Message syntax used in this locale's `messages` map.
    #[serde(default)]
    pub message_syntax: MessageSyntax,
}

/// Vocabulary maps for genre and medium display text.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct VocabMap {
    /// Genre canonical key → display string.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub genre: HashMap<String, String>,
    /// Medium canonical key → display string.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub medium: HashMap<String, String>,
}

impl VocabMap {
    /// Returns true if both maps are empty.
    pub fn is_empty(&self) -> bool {
        self.genre.is_empty() && self.medium.is_empty()
    }
}

/// Partial patch applied on top of a base [`crate::locale::Locale`] for style-specific overrides.
///
/// A `LocaleOverride` allows a style to customize specific messages, grammar options,
/// and legacy term aliases without duplicating the entire base locale. All fields are
/// merged key-by-key into the target locale.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default)]
pub struct LocaleOverride {
    /// Message IDs to replace in the base locale (key-by-key insertion/replacement).
    pub messages: std::collections::HashMap<String, String>,
    /// If present, replaces the entire grammar-options block.
    pub grammar_options: Option<GrammarOptions>,
    /// Additional or replacement legacy term aliases (key-by-key insertion/replacement).
    pub legacy_term_aliases: std::collections::HashMap<String, String>,
    /// Sparse month/season name replacements (key-by-key insertion/replacement).
    pub dates: DateNameOverride,
}

/// Sparse month/season name overrides for a [`LocaleOverride`].
///
/// Reuses [`MonthNames`] and the same EDTF-sub-year-code keying as the base
/// locale's date names, so a style can replace a single month or season
/// name without redeclaring the rest. See
/// `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default)]
pub struct DateNameOverride {
    /// Month name replacements, keyed by EDTF sub-year code (`1`-`12`).
    pub months: MonthNames,
    /// Season name replacements, keyed by EDTF season code (`21`-`24`).
    pub seasons: BTreeMap<SubYearCode, String>,
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

    /// `SubYearCode::new` accepts every month and EDTF Level 1 season code,
    /// and rejects everything else (the gap between them, and the reserved
    /// Level 2 upper bound).
    #[rstest::rstest]
    #[case::first_month(1, true)]
    #[case::last_month(12, true)]
    #[case::first_season(21, true)]
    #[case::last_season_reserved(41, true)]
    #[case::zero_rejected(0, false)]
    #[case::gap_between_months_and_seasons_rejected(13, false)]
    #[case::gap_before_seasons_rejected(20, false)]
    #[case::past_reserved_range_rejected(42, false)]
    fn sub_year_code_range(#[case] code: u8, #[case] expected_valid: bool) {
        assert_eq!(SubYearCode::new(code).is_some(), expected_valid);
    }

    /// `SubYearCode` deserializes from both a YAML/JSON integer and a JSON
    /// object-key string, since JSON forces map keys to strings while YAML
    /// keeps native integer keys — both loaders (`parse_locale_override_bytes`)
    /// must resolve to the identical code.
    #[rstest::rstest]
    #[case::yaml_integer_key("months:\n  7: Jul.\n")]
    #[case::json_string_key(r#"{"months": {"7": "Jul."}}"#)]
    fn sub_year_code_parses_int_or_string_key(#[case] yaml_or_json: &str) {
        #[derive(Deserialize)]
        struct Wrapper {
            months: BTreeMap<SubYearCode, String>,
        }
        let parsed: Wrapper = if yaml_or_json.trim_start().starts_with('{') {
            serde_json::from_str(yaml_or_json).expect("should parse")
        } else {
            serde_yaml::from_str(yaml_or_json).expect("should parse")
        };
        assert_eq!(
            parsed
                .months
                .get(&SubYearCode::new(7).expect("valid month code")),
            Some(&"Jul.".to_string())
        );
    }

    /// An out-of-range sub-year code is a loud deserialize error, not a
    /// silently dropped key.
    #[test]
    fn sub_year_code_rejects_out_of_range_key() {
        let err = serde_yaml::from_str::<BTreeMap<SubYearCode, String>>("13: invalid\n")
            .expect_err("code 13 falls in the gap between months and seasons");
        assert!(err.to_string().contains("out of range"));
    }

    /// Test that GeneralTerm variants deserialize from expected string values.
    #[test]
    fn test_general_term_deserialization() {
        let json_tests = vec![
            (r#""in""#, GeneralTerm::In),
            (r#""accessed""#, GeneralTerm::Accessed),
            (r#""retrieved""#, GeneralTerm::Retrieved),
            (r#""at""#, GeneralTerm::At),
            (r#""from""#, GeneralTerm::From),
            (r#""of""#, GeneralTerm::Of),
            (r#""to""#, GeneralTerm::To),
            (r#""by""#, GeneralTerm::By),
            (r#""no-date""#, GeneralTerm::NoDate),
            (r#""anonymous""#, GeneralTerm::Anonymous),
            (r#""circa""#, GeneralTerm::Circa),
            (r#""available-at""#, GeneralTerm::AvailableAt),
            (r#""ibid""#, GeneralTerm::Ibid),
            (r#""and""#, GeneralTerm::And),
            (r#""et-al""#, GeneralTerm::EtAl),
            (r#""and-others""#, GeneralTerm::AndOthers),
            (r#""forthcoming""#, GeneralTerm::Forthcoming),
            (r#""online""#, GeneralTerm::Online),
            (r#""here""#, GeneralTerm::Here),
            (r#""deposited""#, GeneralTerm::Deposited),
            (r#""review-of""#, GeneralTerm::ReviewOf),
            (
                r#""original-work-published""#,
                GeneralTerm::OriginalWorkPublished,
            ),
            (r#""patent""#, GeneralTerm::Patent),
            (r#""issued""#, GeneralTerm::Issued),
            (r#""volume""#, GeneralTerm::Volume),
            (r#""issue""#, GeneralTerm::Issue),
            (r#""page""#, GeneralTerm::Page),
            (r#""chapter""#, GeneralTerm::Chapter),
            (r#""edition""#, GeneralTerm::Edition),
            (r#""section""#, GeneralTerm::Section),
        ];

        for (json_str, expected) in json_tests {
            let result: GeneralTerm = serde_json::from_str(json_str)
                .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", json_str, e));
            assert_eq!(result, expected, "Mismatch for {}", json_str);
        }
    }

    /// Test that TermForm variants deserialize from expected string values.
    #[test]
    fn test_term_form_deserialization() {
        let form_long: TermForm = serde_json::from_str(r#""long""#).unwrap();
        assert_eq!(form_long, TermForm::Long);

        let form_short: TermForm = serde_json::from_str(r#""short""#).unwrap();
        assert_eq!(form_short, TermForm::Short);

        let form_verb: TermForm = serde_json::from_str(r#""verb""#).unwrap();
        assert_eq!(form_verb, TermForm::Verb);

        let form_verb_short: TermForm = serde_json::from_str(r#""verb-short""#).unwrap();
        assert_eq!(form_verb_short, TermForm::VerbShort);

        let form_symbol: TermForm = serde_json::from_str(r#""symbol""#).unwrap();
        assert_eq!(form_symbol, TermForm::Symbol);
    }

    /// Test that SimpleTerm can be constructed and provides both forms.
    #[test]
    fn test_simple_term_construction() {
        let term = SimpleTerm {
            long: "anonymous".into(),
            short: "anon.".into(),
        };

        assert_eq!(term.long, MaybeGendered::Plain("anonymous".to_string()));
        assert_eq!(term.short, MaybeGendered::Plain("anon.".to_string()));
    }

    /// Test that SingularPlural provides both singular and plural forms.
    #[test]
    fn test_singular_plural_construction() {
        let term = SingularPlural {
            singular: "page".into(),
            plural: "pages".into(),
        };

        assert_eq!(term.singular, MaybeGendered::Plain("page".to_string()));
        assert_eq!(term.plural, MaybeGendered::Plain("pages".to_string()));
    }

    /// The legacy no-date fallback (`no_date`) must not serialize alongside
    /// the structured `no-date` entry in `general`, even though both are
    /// populated (as they are for a fully-loaded en-US locale).
    #[test]
    fn test_terms_serializes_single_no_date_entry() {
        let terms = Terms {
            no_date: Some("n.d.".to_string()),
            general: std::collections::HashMap::from([(
                GeneralTerm::NoDate,
                SimpleTerm {
                    long: "no date".into(),
                    short: "n.d.".into(),
                },
            )]),
            ..Default::default()
        };
        let value = serde_json::to_value(&terms).unwrap();
        let object = value.as_object().unwrap();

        assert_eq!(
            object.get("no-date"),
            Some(&serde_json::json!({
                "long": "no date",
                "short": "n.d."
            }))
        );
        assert_eq!(object.get("no_date"), None);
    }

    /// The YAML-derived `Locale::en_us()` provides the same general terms
    /// that the deleted hardcoded `Terms::en_us()` constructor used to.
    #[test]
    fn test_locale_en_us_terms_defaults() {
        let locale = super::super::Locale::en_us();
        let terms = &locale.terms;

        assert_eq!(terms.and, Some("and".to_string()));
        assert_eq!(terms.and_symbol, Some("&".to_string()));
        assert_eq!(terms.and_others, Some("and others".to_string()));
        assert_eq!(terms.et_al, Some("et al.".to_string()));
        assert_eq!(terms.ibid, Some("ibid.".to_string()));

        // "circa" is parsed into the flattened `general` map (keyed by
        // GeneralTerm), not the legacy dedicated `circa` field, so resolve
        // it via the same public API callers use.
        assert_eq!(
            locale.general_term(&GeneralTerm::Circa, &TermForm::Long, None),
            Some("circa")
        );
        assert_eq!(
            locale.general_term(&GeneralTerm::Circa, &TermForm::Short, None),
            Some("c.")
        );
    }

    /// The YAML-derived `Locale::en_us()` provides month names and seasons
    /// that the deleted hardcoded `DateTerms::en_us()` constructor used to.
    #[test]
    fn test_locale_en_us_dates_months_and_seasons() {
        let dates = &super::super::Locale::en_us().dates;

        assert_eq!(dates.months.long.len(), 12);
        assert_eq!(dates.months.short.len(), 12);
        assert_eq!(
            dates.months.long[&SubYearCode::new(1).expect("valid month code")],
            "January"
        );
        assert_eq!(
            dates.months.long[&SubYearCode::new(12).expect("valid month code")],
            "December"
        );

        assert_eq!(dates.seasons.len(), 4);
        assert_eq!(
            dates.seasons[&SubYearCode::new(21).expect("valid season code")],
            "Spring"
        );
        assert_eq!(
            dates.seasons[&SubYearCode::new(22).expect("valid season code")],
            "Summer"
        );
        assert_eq!(
            dates.seasons[&SubYearCode::new(23).expect("valid season code")],
            "Autumn"
        );
        assert_eq!(
            dates.seasons[&SubYearCode::new(24).expect("valid season code")],
            "Winter"
        );
    }

    /// The YAML-derived `Locale::en_us()` provides era suffixes that the
    /// deleted hardcoded `DateTerms::en_us()` constructor used to.
    #[test]
    fn test_locale_en_us_dates_before_era() {
        let dates = &super::super::Locale::en_us().dates;

        assert_eq!(dates.before_era.as_deref(), Some("BC"));
        assert_eq!(dates.ad.as_deref(), Some("AD"));
        assert_eq!(dates.bc.as_deref(), Some("BC"));
        assert_eq!(dates.bce.as_deref(), Some("BCE"));
        assert_eq!(dates.ce.as_deref(), Some("CE"));
    }
}
