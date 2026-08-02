/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Locale definitions for Citum.
//!
//! Locales provide language-specific terms, date formats, and punctuation rules
//! for citation formatting.

mod date_patterns;
/// Locator text normalization.
pub mod locator;
/// Message evaluation for parameterized locale strings.
pub mod message;
mod message_ids;
/// Raw locale types used during locale file parsing.
pub mod raw;
mod raw_conversion;
mod sort;
mod terms;
/// Structured locale types used by the processor.
pub mod types;
mod vocab;

use crate::citation::LocatorType;
use crate::template::ContributorRole;
pub use message::{MessageArgs, MessageEvaluator, Mf2MessageEvaluator};
pub use raw::{RawLocale, RawTermValue};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
pub use terms::ArchiveHierarchyField;
pub use types::*;

/// A list of month names (12 elements for Jan-Dec).
pub type MonthList = Vec<String>;

/// A locale definition containing language-specific terms and formatting rules.
///
/// The `evaluator` field holds the message evaluation engine, selected based on
/// `evaluation.message_syntax`. This allows for trait-based swapping to ICU4X
/// implementations in the future without changing call sites.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Locale {
    /// The locale identifier (e.g., "en-US", "de-DE").
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub locale: String,
    /// Date-related terms (months, seasons).
    #[serde(default)]
    pub dates: DateTerms,
    /// Contributor role terms (editor, translator, etc.).
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub roles: HashMap<ContributorRole, ContributorTerm>,
    /// Authored terms for combinations such as `writer-director`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub role_combinations: HashMap<String, ContributorTerm>,
    /// Locator terms (page, chapter, etc.).
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub locators: HashMap<LocatorType, LocatorTerm>,
    /// General terms (and, et al., etc.).
    #[serde(default)]
    pub terms: Terms,
    /// Whether to place periods/commas inside quotation marks.
    /// true = American style ("text."), false = British style ("text".)
    #[serde(default)]
    pub punctuation_in_quote: bool,
    /// Articles to strip from titles when sorting (e.g., "the", "a", "an" for English).
    /// These should be lowercase and will be matched case-insensitively.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort_articles: Vec<String>,
    /// Schema version from the source locale file (None = legacy v1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale_schema_version: Option<String>,
    /// Runtime evaluation configuration.
    #[serde(default)]
    pub evaluation: EvaluationConfig,
    /// ICU MF1 messages keyed by message ID (populated for v2 locales).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub messages: HashMap<String, String>,
    /// Named date format presets: symbolic name → CLDR pattern.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub date_formats: HashMap<String, String>,
    /// Number formatting options.
    #[serde(default)]
    pub number_formats: NumberFormats,
    /// Grammar options.
    #[serde(default)]
    pub grammar_options: GrammarOptions,
    /// Partial semantic punctuation realization table owned by this locale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub punctuation_realization: Option<crate::options::PunctuationRealization>,
    /// Backwards-compatibility aliases: old term key → new message ID.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub legacy_term_aliases: HashMap<String, String>,
    /// Vocabulary maps for genre and medium display text.
    #[serde(default, skip_serializing_if = "VocabMap::is_empty")]
    pub vocab: VocabMap,
    /// Reference-type description terms, keyed by CSL-style `ref_type`
    /// spelling (e.g. `"dataset"`, `"article-journal"`). Used by the
    /// `type-label` template component to resolve a localized fallback
    /// label when a reference has no `genre`/`medium` override. See
    /// `docs/specs/TYPE_CLASSIFICATION_CENTRALIZATION.md`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub type_terms: HashMap<String, SimpleTerm>,
    /// Message evaluator implementation (not serialized; set during load).
    #[serde(skip, default = "default_evaluator")]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub evaluator: Arc<dyn MessageEvaluator>,
}

/// Default message evaluator (MF2).
fn default_evaluator() -> Arc<dyn MessageEvaluator> {
    Arc::new(Mf2MessageEvaluator)
}

impl Default for Locale {
    fn default() -> Self {
        Self {
            locale: String::default(),
            dates: DateTerms::default(),
            roles: HashMap::default(),
            role_combinations: HashMap::default(),
            locators: HashMap::default(),
            terms: Terms::default(),
            punctuation_in_quote: false,
            sort_articles: Vec::default(),
            locale_schema_version: None,
            evaluation: EvaluationConfig::default(),
            messages: HashMap::default(),
            date_formats: HashMap::default(),
            number_formats: NumberFormats::default(),
            grammar_options: GrammarOptions::default(),
            punctuation_realization: None,
            legacy_term_aliases: HashMap::default(),
            vocab: VocabMap::default(),
            type_terms: HashMap::default(),
            evaluator: default_evaluator(),
        }
    }
}

impl fmt::Debug for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Locale")
            .field("locale", &self.locale)
            .field("dates", &self.dates)
            .field("roles", &self.roles)
            .field("role_combinations", &self.role_combinations)
            .field("locators", &self.locators)
            .field("terms", &self.terms)
            .field("punctuation_in_quote", &self.punctuation_in_quote)
            .field("sort_articles", &self.sort_articles)
            .field("locale_schema_version", &self.locale_schema_version)
            .field("evaluation", &self.evaluation)
            .field("messages", &self.messages)
            .field("date_formats", &self.date_formats)
            .field("number_formats", &self.number_formats)
            .field("grammar_options", &self.grammar_options)
            .field("punctuation_realization", &self.punctuation_realization)
            .field("legacy_term_aliases", &self.legacy_term_aliases)
            .field("vocab", &self.vocab)
            .field("type_terms", &self.type_terms)
            .field("evaluator", &"<MessageEvaluator>")
            .finish()
    }
}

impl Locale {
    /// Create the English (US) locale, the fallback baseline every other
    /// locale inherits from and the default for the majority of embedded
    /// styles (which declare no `info.default-locale`).
    ///
    /// This parses the embedded canonical asset
    /// (`embedded/locales/en-US.yaml`) so the YAML is the single source of
    /// truth — there is no separate hand-maintained Rust copy to drift out
    /// of sync with it. The parse is memoized in a `std::sync::OnceLock`
    /// since it is pure and immutable; callers get a `clone()` of the cached
    /// result (still a deep copy of its maps/vecs, but far cheaper than
    /// re-parsing the YAML) rather than re-parsing on every call.
    ///
    /// Seeds from [`Locale::default()`] (not `from_raw`'s usual
    /// `Locale::en_us()` seed) via `from_raw_with_base` to avoid infinite
    /// recursion through this very function.
    ///
    /// # Panics
    ///
    /// Panics if the embedded `en-US.yaml` asset is missing, not valid
    /// UTF-8, or fails to parse. This cannot happen at runtime: the asset is
    /// embedded at compile time and covered by
    /// `bundled_ar_ar_and_eu_es_locales_are_embedded_and_parseable`-style
    /// tests, so a failure here indicates a broken build, not bad input.
    #[allow(
        clippy::expect_used,
        reason = "Embedded en-US.yaml locale must parse; failure indicates a broken build, not bad input"
    )]
    pub fn en_us() -> Self {
        static EN_US: std::sync::OnceLock<Locale> = std::sync::OnceLock::new();
        EN_US
            .get_or_init(|| {
                let bytes = crate::embedded::get_locale_bytes("en-US")
                    .expect("en-US is a compile-time embedded locale");
                let yaml = std::str::from_utf8(bytes).expect("embedded en-US.yaml is valid UTF-8");
                let raw: RawLocale =
                    serde_yaml::from_str(yaml).expect("embedded en-US.yaml parses");
                Self::from_raw_with_base(raw, Locale::default())
            })
            .clone()
    }

    /// Create the Québec French locale by applying its regional typography to
    /// the bundled French lexical locale.
    ///
    /// # Panics
    ///
    /// Panics if either embedded French locale asset fails to parse, which
    /// indicates a broken build rather than invalid runtime input.
    #[allow(
        clippy::expect_used,
        reason = "Embedded French locale assets must parse; failure indicates a broken build"
    )]
    #[must_use]
    pub fn fr_ca() -> Self {
        let mut french = Self::from_yaml_str(include_str!("../../embedded/locales/fr-FR.yaml"))
            .expect("embedded fr-FR.yaml parses");
        let raw: RawLocale =
            serde_yaml::from_str(include_str!("../../embedded/locales/fr-CA.yaml"))
                .expect("embedded fr-CA.yaml parses");
        french.locale = raw.locale;
        french.locale_schema_version = raw.locale_schema_version;
        french.date_formats.extend(raw.date_formats);
        if let Some(grammar_options) = raw.grammar_options {
            french.punctuation_in_quote = grammar_options.punctuation_in_quote;
            french.grammar_options = grammar_options;
        }
        french.punctuation_realization = raw.punctuation_realization;
        french
    }

    /// Build a rendering locale that speaks `item`'s terms, roles, locators,
    /// messages, and date names/patterns inside `self`'s (the style's)
    /// typography and identity.
    ///
    /// This is the `options.multilingual.term-locale: item` hybrid: "terms
    /// are the item speaking; typography is the document speaking" (see
    /// `docs/specs/PER_ITEM_TERM_LOCALE.md` §4). The field list is written
    /// out explicitly, not built by cloning `self` and overwriting a few
    /// fields, so that a field added to `Locale` later must be placed on one
    /// side of the split deliberately rather than silently inheriting the
    /// wrong one.
    #[must_use]
    pub fn with_term_surfaces_from(&self, item: &Locale) -> Locale {
        Locale {
            // Identity and typography: stay with the style locale. The id
            // is also read as a data-translation target (multilingual
            // titles/archive names) and for term-casing tailoring; both
            // uses are out of scope for this switch (§4).
            locale: self.locale.clone(),
            punctuation_in_quote: self.punctuation_in_quote,
            sort_articles: self.sort_articles.clone(),
            locale_schema_version: self.locale_schema_version.clone(),
            number_formats: self.number_formats.clone(),
            grammar_options: self.grammar_options.clone(),
            punctuation_realization: item.punctuation_realization.clone(),
            // Word and date surfaces: switch to the item locale.
            dates: item.dates.clone(),
            roles: item.roles.clone(),
            role_combinations: item.role_combinations.clone(),
            locators: item.locators.clone(),
            terms: item.terms.clone(),
            evaluation: item.evaluation.clone(),
            messages: item.messages.clone(),
            date_formats: item.date_formats.clone(),
            legacy_term_aliases: item.legacy_term_aliases.clone(),
            vocab: item.vocab.clone(),
            type_terms: item.type_terms.clone(),
            evaluator: item.evaluator.clone(),
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
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_en_us_locale_model_defaults() {
        let locale = Locale::en_us();
        assert_eq!(locale.locale, "en-US");
        assert!(locale.punctuation_in_quote);
        assert_eq!(locale.sort_articles, ["the", "a", "an"]);
        assert!(locale.roles.contains_key(&ContributorRole::Editor));
        assert!(locale.locators.contains_key(&LocatorType::Page));
    }

    #[test]
    fn test_locale_deserialization() {
        // `Locale` derives `Deserialize` directly for its own canonical
        // round-trip format (e.g. cache/IPC), which is always the
        // EDTF-sub-year-code-keyed map — distinct from the raw-file loader
        // (`RawLocale`/`from_yaml_str`), which additionally accepts the
        // legacy sequence form. See `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
        let json = r#"{
            "locale": "en-US",
            "dates": {
                "months": {
                    "long": {
                        "1": "January", "2": "February", "3": "March", "4": "April",
                        "5": "May", "6": "June", "7": "July", "8": "August",
                        "9": "September", "10": "October", "11": "November", "12": "December"
                    },
                    "short": {
                        "1": "Jan", "2": "Feb", "3": "Mar", "4": "Apr", "5": "May", "6": "Jun",
                        "7": "Jul", "8": "Aug", "9": "Sep", "10": "Oct", "11": "Nov", "12": "Dec"
                    }
                },
                "seasons": {"21": "Spring", "22": "Summer", "23": "Autumn", "24": "Winter"}
            },
            "roles": {},
            "terms": {
                "and": "and",
                "et-al": "et al."
            }
        }"#;

        let locale: Locale = serde_json::from_str(json).unwrap();
        assert_eq!(locale.locale, "en-US");
        assert_eq!(
            locale.dates.months.long[&SubYearCode::new(1).expect("valid month code")],
            "January"
        );
        assert_eq!(locale.terms.and.as_ref().unwrap(), "and");
    }

    #[test]
    fn locale_punctuation_realization_deserializes_as_a_partial_table() {
        let locale = Locale::from_yaml_str(
            r#"
locale: test
punctuation-realization:
  colon: "\u00A0: "
"#,
        )
        .expect("locale punctuation realization should parse");

        let realization = locale
            .punctuation_realization
            .expect("locale punctuation realization should be present");
        assert_eq!(realization.colon.as_deref(), Some("\u{a0}: "));
        assert_eq!(realization.semicolon, None);
    }

    #[test]
    fn fr_ca_inherits_french_lexical_data_and_overrides_punctuation_realization() {
        let locale = Locale::fr_ca();

        assert_eq!(locale.locale, "fr-CA");
        assert_eq!(
            locale
                .dates
                .months
                .long
                .get(&SubYearCode::new(1).expect("valid month code"))
                .map(String::as_str),
            Some("janvier")
        );
        assert_eq!(
            locale
                .punctuation_realization
                .as_ref()
                .and_then(|table| table.semicolon.as_deref()),
            Some("; ")
        );
    }

    #[test]
    fn test_yaml_locale_loading() {
        let yaml = r#"
locale: de-DE
dates:
  months:
    long:
      - Januar
      - Februar
      - März
      - April
      - Mai
      - Juni
      - Juli
      - August
      - September
      - Oktober
      - November
      - Dezember
    short:
      - Jan.
      - Feb.
      - März
      - Apr.
      - Mai
      - Juni
      - Juli
      - Aug.
      - Sep.
      - Okt.
      - Nov.
      - Dez.
  seasons:
    - Frühling
    - Sommer
    - Herbst
    - Winter
terms:
  and:
    long: und
    symbol: "&"
  et_al:
    long: "u. a."
"#;

        let locale = Locale::from_yaml_str(yaml).unwrap();
        assert_eq!(locale.locale, "de-DE");
        assert_eq!(locale.terms.and.as_deref(), Some("und"));
        assert_eq!(locale.terms.et_al.as_deref(), Some("u. a."));
        assert_eq!(
            locale.dates.months.long[&SubYearCode::new(1).expect("valid month code")],
            "Januar"
        );
        assert_eq!(
            locale.dates.months.long[&SubYearCode::new(3).expect("valid month code")],
            "März"
        );
    }

    /// v2 locale with grammar-options overrides punctuation_in_quote correctly.
    #[test]
    fn test_v2_grammar_options_sync_punctuation_in_quote() {
        let yaml = r#"
locale-schema-version: "2"
locale: en-GB
grammar-options:
  punctuation-in-quote: false
"#;
        let locale = Locale::from_yaml_str(yaml).unwrap();
        // grammar_options is the authoritative source for v2 locales
        assert!(!locale.grammar_options.punctuation_in_quote);
        // legacy field is synced from grammar_options
        assert!(!locale.punctuation_in_quote);
    }

    /// v1 locale (no grammar-options) derives punctuation_in_quote from locale ID.
    #[test]
    fn test_v1_locale_derives_punctuation_from_locale_id() {
        let yaml = r#"
locale: en-US
"#;
        let locale = Locale::from_yaml_str(yaml).unwrap();
        // en-US uses American style (inside)
        assert!(locale.punctuation_in_quote);
        assert!(locale.grammar_options.punctuation_in_quote);
    }

    /// Partial locales inherit base messages, date formats, and aliases.
    #[test]
    fn test_partial_locale_merges_raw_maps_with_base() {
        let yaml = r#"
locale-schema-version: "2"
locale: zz-ZZ
messages:
  pattern.in-container: "inside {$container}"
date-formats:
  numeric-short: "dd/MM/y"
locators:
  page:
    long:
      singular: page-localized
      plural: pages-localized
legacy-term-aliases:
  page: term.page-label-long
"#;
        let locale = Locale::from_yaml_str(yaml).unwrap();

        assert_eq!(
            locale
                .messages
                .get("pattern.originally-published-as")
                .map(String::as_str),
            Some("originally published as {$title}")
        );
        assert_eq!(
            locale
                .messages
                .get("pattern.in-container")
                .map(String::as_str),
            Some("inside {$container}")
        );
        assert_eq!(
            locale.date_formats.get("textual-full").map(String::as_str),
            Some("MMMM d, yyyy")
        );
        assert_eq!(
            locale.date_formats.get("numeric-short").map(String::as_str),
            Some("dd/MM/y")
        );
        assert_eq!(
            locale.legacy_term_aliases.get("and").map(String::as_str),
            Some("term.and")
        );
        assert_eq!(
            locale.legacy_term_aliases.get("page").map(String::as_str),
            Some("term.page-label-long")
        );
        assert_eq!(
            locale.resolved_locator_term(&LocatorType::Page, false, &TermForm::Long, None),
            Some("page-localized".to_string())
        );
    }

    /// apply_override merges messages key-by-key into the base locale.
    #[test]
    fn test_apply_override_merges_messages() {
        let mut locale = Locale::en_us();
        locale
            .messages
            .insert("term.page-label".into(), "p.".into());
        let ov = LocaleOverride {
            messages: [("term.page-label".into(), "pg.".into())].into(),
            ..Default::default()
        };
        locale.apply_override(&ov);
        assert_eq!(
            locale.messages.get("term.page-label").map(|s| s.as_str()),
            Some("pg.")
        );
    }

    /// The hardcoded en-US locale includes phrase messages used by style
    /// `message:` components, not only legacy term compatibility messages.
    #[test]
    fn test_en_us_locale_resolves_phrase_messages() {
        let locale = Locale::en_us();
        let args = MessageArgs {
            named: [("container".to_string(), "Book Title".to_string())].into(),
            ..Default::default()
        };

        assert_eq!(
            locale.resolve_message("pattern.in-container", &args),
            Some("in Book Title".to_string())
        );
    }

    /// apply_override with grammar_options replaces block and syncs punctuation_in_quote.
    #[test]
    fn test_apply_override_grammar_options_syncs_punctuation() {
        let mut locale = Locale::en_us();
        locale.punctuation_in_quote = false;
        let ov = LocaleOverride {
            grammar_options: Some(GrammarOptions {
                punctuation_in_quote: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        locale.apply_override(&ov);
        assert!(locale.punctuation_in_quote);
        assert!(locale.grammar_options.punctuation_in_quote);
    }

    /// `apply_override` replaces only the named month, leaving the other
    /// eleven (and all four seasons) untouched — the bean's core ask:
    /// a style overrides one abbreviation without redeclaring the rest.
    #[test]
    fn test_apply_override_merges_single_month_name() {
        let mut locale = Locale::en_us();
        let july = SubYearCode::new(7).expect("valid month code");
        let june = SubYearCode::new(6).expect("valid month code");

        let ov = LocaleOverride {
            dates: DateNameOverride {
                months: MonthNames {
                    long: BTreeMap::new(),
                    short: [(july, "Jul.".to_string())].into(),
                },
                seasons: BTreeMap::new(),
            },
            ..Default::default()
        };
        locale.apply_override(&ov);

        assert_eq!(locale.dates.months.short[&july], "Jul.");
        assert_eq!(locale.dates.months.short[&june], "June");
        assert_eq!(locale.dates.months.long[&july], "July");
    }

    /// An out-of-range override key never reaches `apply_override` — it is
    /// rejected at deserialize time (see `sub_year_code_rejects_out_of_range_key`
    /// in `types.rs`), so `apply_override` itself has nothing to validate.
    /// This test instead confirms a season override merges independently of
    /// the month tables.
    #[test]
    fn test_apply_override_merges_season_name_independent_of_months() {
        let mut locale = Locale::en_us();
        let spring = SubYearCode::new(21).expect("valid season code");

        let ov = LocaleOverride {
            dates: DateNameOverride {
                months: MonthNames::default(),
                seasons: [(spring, "Printemps".to_string())].into(),
            },
            ..Default::default()
        };
        locale.apply_override(&ov);

        assert_eq!(locale.dates.seasons[&spring], "Printemps");
        assert_eq!(locale.dates.months.long.len(), 12);
    }

    /// Every embedded locale round-trips through raw-YAML parsing to the
    /// canonical EDTF-sub-year-code-keyed map without losing or duplicating
    /// a month or season name, regardless of whether the source YAML uses
    /// the legacy sequence form or the canonical map form.
    #[test]
    fn embedded_locales_have_complete_keyed_month_and_season_tables() {
        // ar-AR has no authored short-month forms in its source YAML
        // (a pre-existing content gap, unrelated to this keying change);
        // every other bundled locale defines both full and abbreviated
        // forms for all 12 months.
        const NO_SHORT_MONTHS: &[&str] = &["ar-AR"];

        for &id in crate::embedded::EMBEDDED_LOCALE_IDS {
            let locale = crate::embedded::get_locale(id)
                .unwrap_or_else(|| panic!("{id} should be embedded"));

            assert_eq!(locale.dates.months.long.len(), 12, "{id} long months");
            assert_eq!(locale.dates.seasons.len(), 4, "{id} seasons");
            if !NO_SHORT_MONTHS.contains(&id) {
                assert_eq!(locale.dates.months.short.len(), 12, "{id} short months");
            }

            for code in 1..=12u8 {
                let key = SubYearCode::new(code).expect("valid month code");
                assert!(
                    locale.dates.months.long.contains_key(&key),
                    "{id} missing long month {code}"
                );
            }
            for code in 21..=24u8 {
                let key = SubYearCode::new(code).expect("valid season code");
                assert!(
                    locale.dates.seasons.contains_key(&key),
                    "{id} missing season {code}"
                );
            }
        }
    }

    #[test]
    fn embedded_locale_ids_include_all_bundled_locale_files() {
        for id in [
            "en-US", "ar-AR", "de-DE", "es-ES", "eu-ES", "fr-FR", "tr-TR", "zh-CN", "ja-JP",
            "ko-KR", "ru-RU",
        ] {
            assert!(
                crate::embedded::EMBEDDED_LOCALE_IDS.contains(&id),
                "{id} should be listed as an embedded locale"
            );
        }
    }

    #[test]
    fn bundled_ar_ar_and_eu_es_locales_are_embedded_and_parseable() {
        for id in ["ar-AR", "eu-ES"] {
            let bytes = crate::embedded::get_locale_bytes(id).expect("locale should be embedded");
            let yaml = std::str::from_utf8(bytes).expect("embedded locale should be utf-8");
            let locale = Locale::from_yaml_str(yaml).expect("embedded locale should parse");

            assert_eq!(locale.locale, id);
        }
    }

    /// Round-trip regression guard for the new ja-JP/ko-KR/ru-RU locales
    /// (`csl26-tfi8`): parses each embedded file and spot-checks a handful
    /// of the values a future edit to that YAML could silently regress.
    #[test]
    fn bundled_ja_jp_ko_kr_ru_ru_locales_are_embedded_and_parseable() {
        for (id, editor_short, and_term) in [
            ("ja-JP", "編", "と"),
            ("ko-KR", "편", "및"),
            ("ru-RU", "ред.", "и"),
        ] {
            let bytes = crate::embedded::get_locale_bytes(id).expect("locale should be embedded");
            let yaml = std::str::from_utf8(bytes).expect("embedded locale should be utf-8");
            let locale = Locale::from_yaml_str(yaml).expect("embedded locale should parse");

            assert_eq!(locale.locale, id);
            assert_eq!(
                locale.resolved_role_term(&ContributorRole::Editor, false, &TermForm::Short, None),
                Some(editor_short.to_string()),
                "{id} editor short-form role term"
            );
            assert_eq!(
                locale.resolved_general_term(&GeneralTerm::And, &TermForm::Long, None),
                Some(and_term.to_string()),
                "{id} 'and' term"
            );
            assert!(
                locale.date_formats.contains_key("iso"),
                "{id} should carry date-formats"
            );
        }
    }

    /// CI enforcement for the locale-completeness lint (`csl26-itri`): every
    /// embedded v2 locale must ship `grammar-options` and `date-formats`, or
    /// its typography/dates silently fall back to English. Scoped to just
    /// the two completeness findings (not general lint errors) so this test
    /// doesn't couple to unrelated pre-existing lint issues in other
    /// embedded locales.
    #[test]
    fn embedded_v2_locales_pass_completeness_lint() {
        for &id in crate::embedded::EMBEDDED_LOCALE_IDS {
            let bytes = crate::embedded::get_locale_bytes(id).expect("locale should be embedded");
            let raw: RawLocale =
                serde_yaml::from_slice(bytes).expect("embedded locale should parse as RawLocale");

            if raw.locale_schema_version.as_deref() != Some("2") {
                continue;
            }

            let report = crate::lint::lint_raw_locale(&raw);
            assert!(
                !report
                    .findings
                    .iter()
                    .any(|finding| finding.path == "grammar-options"),
                "{id} is missing grammar-options"
            );
            assert!(
                !report
                    .findings
                    .iter()
                    .any(|finding| finding.path == "date-formats"),
                "{id} is missing date-formats"
            );
        }
    }

    /// Round-trip regression guard for `Locale::en_us()` parsing the
    /// embedded `en-US.yaml` asset: asserts the critical values a future
    /// edit to that YAML could silently regress, since `en_us()` is the
    /// fallback baseline for the large majority of embedded styles.
    #[test]
    fn en_us_locale_round_trip_carries_critical_values() {
        let locale = Locale::en_us();

        // Role labels (CSL reference: scripts/locales-en-US.xml).
        assert_eq!(
            locale.resolved_role_term(&ContributorRole::Translator, false, &TermForm::Short, None),
            Some("trans.".to_string())
        );

        // Locator labels (CSL reference: chap./chaps.).
        assert_eq!(
            locale.locator_term(&LocatorType::Chapter, false, &TermForm::Short, None),
            Some("chap.")
        );
        assert_eq!(
            locale.locator_term(&LocatorType::Chapter, true, &TermForm::Short, None),
            Some("chaps.")
        );

        // No-date term is form-aware (see general_term fix).
        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Long, None),
            Some("no date")
        );
        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Short, None),
            Some("n.d.")
        );

        // Core general terms.
        assert_eq!(locale.terms.and.as_deref(), Some("and"));
        assert_eq!(locale.terms.et_al.as_deref(), Some("et al."));

        // Month names.
        assert_eq!(
            locale
                .dates
                .months
                .long
                .get(&SubYearCode::new(1).expect("valid month code"))
                .map(String::as_str),
            Some("January")
        );

        // Number formats (single-sourced explicitly in the YAML, Step 4).
        assert_eq!(locale.number_formats.decimal_separator, ".");
        assert_eq!(locale.number_formats.thousands_separator, ",");
        assert_eq!(locale.number_formats.minimum_digits, 1);
        assert_eq!(locale.number_formats.digit_system, DigitSystem::Western);

        // Sort articles.
        assert_eq!(locale.sort_articles, ["the", "a", "an"]);
    }

    #[test]
    fn locale_number_formats_accept_each_supported_digit_system() {
        for (digit_system, expected) in [
            ("western", DigitSystem::Western),
            ("arabic-indic", DigitSystem::ArabicIndic),
            ("extended-arabic-indic", DigitSystem::ExtendedArabicIndic),
            ("devanagari", DigitSystem::Devanagari),
        ] {
            let locale = Locale::from_yaml_str(&format!(
                "locale: test\nnumber-formats:\n  digit-system: {digit_system}\n"
            ))
            .expect("locale should parse");

            assert_eq!(locale.number_formats.digit_system, expected);
        }
    }
}
