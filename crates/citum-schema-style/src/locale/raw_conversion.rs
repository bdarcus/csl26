/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Conversion from [`raw::RawLocale`] (serde-facing schema) into the runtime
//! [`Locale`] type, plus the file/YAML/JSON/CBOR loaders that drive it.
//!
//! Parsing helpers (`extract_*`, `parse_*`, `from_raw_gendered_string`) live
//! alongside `from_raw` so a reader sees the whole transformation in one
//! place. Public Locale APIs unrelated to raw conversion stay in `mod.rs`.

use super::Locale;
use super::message::{MessageEvaluator, Mf2MessageEvaluator, NoOpEvaluator};
use super::raw;
use super::types::{
    ContributorTerm, DateTerms, LocaleOverride, LocatorTerm, MaybeGendered, MessageSyntax,
    MonthNames, SimpleTerm, SingularPlural, SubYearCode, TermForm,
};
use crate::citation::LocatorType;
use crate::template::ContributorRole;
use std::collections::HashMap;
use std::sync::Arc;

impl Locale {
    /// Load a locale from a YAML string.
    ///
    /// # Errors
    ///
    /// Returns an error when the YAML cannot be parsed into a locale.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, String> {
        let raw: raw::RawLocale = serde_yaml::from_str(yaml)
            .map_err(|e| format!("Failed to parse locale YAML: {}", e))?;

        Ok(Self::from_raw(raw))
    }

    /// Load a locale by ID (e.g., "en-US", "de-DE") from a locales directory.
    /// Falls back to en-US if the locale file is not found.
    pub fn load(locale_id: &str, locales_dir: &std::path::Path) -> Self {
        let extensions = ["yaml", "yml", "json", "cbor"];

        for ext in &extensions {
            let file_name = format!("{}.{}", locale_id, ext);
            let file_path = locales_dir.join(&file_name);

            if file_path.exists() {
                match Self::from_file(&file_path) {
                    Ok(locale) => return locale.resolved_for(locale_id),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load locale {}.{}: {}",
                            locale_id, ext, e
                        );
                    }
                }
            }
        }

        if locale_id.contains('-') {
            let base = locale_id.split('-').next().unwrap_or("en");
            if let Ok(entries) = std::fs::read_dir(locales_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if (name_str.starts_with(base)
                        && extensions.iter().any(|ext| name_str.ends_with(ext)))
                        && let Ok(locale) = Self::from_file(&entry.path())
                    {
                        return locale.resolved_for(locale_id);
                    }
                }
            }
        }

        Self::en_us().resolved_for(locale_id)
    }

    /// Load locale from a file path directly (detects format).
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or its contents cannot be
    /// parsed as a supported locale format.
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("Failed to read locale file: {}", e))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

        match ext {
            "cbor" => ciborium::de::from_reader::<raw::RawLocale, _>(std::io::Cursor::new(&bytes))
                .map(Self::from_raw)
                .map_err(|e| format!("Failed to parse CBOR locale: {}", e)),
            "json" => serde_json::from_slice::<raw::RawLocale>(&bytes)
                .map(Self::from_raw)
                .map_err(|e| format!("Failed to parse JSON locale: {}", e)),
            _ => {
                let content = String::from_utf8_lossy(&bytes);
                Self::from_yaml_str(&content)
            }
        }
    }

    /// Convert a RawLocale to a Locale, seeding from [`Locale::en_us()`].
    ///
    /// This is the inheritance model non-en-US locales rely on: a partial
    /// locale file (e.g. `ar-AR`, `eu-ES`) only overrides the fields it
    /// specifies, and everything else falls back to the English baseline —
    /// but the inheritance is field-granular, not uniform. `roles`,
    /// `locators`, `terms`, `vocab`, `messages`, `date_formats`, and
    /// `legacy_term_aliases` are merged key-by-key into the base, so an
    /// omitted key keeps the base's value while an explicit raw entry
    /// overrides it. Inherited message fallbacks do not shadow structured
    /// terms supplied by the raw locale.
    fn from_raw(raw: raw::RawLocale) -> Self {
        Self::from_raw_with_base(raw, Locale::en_us())
    }

    /// Convert a RawLocale to a Locale, seeding from the given `base` locale
    /// instead of always inheriting from [`Locale::en_us()`].
    ///
    /// This exists so [`Locale::en_us()`] itself can parse the embedded
    /// `en-US.yaml` without infinite recursion: it seeds from
    /// [`Locale::default()`] (a non-circular, fully-formed empty locale)
    /// rather than from `en_us()`. Non-en-US locale loading is unaffected —
    /// [`Locale::from_raw`] still calls this with `Locale::en_us()` as the
    /// base, preserving partial-locale inheritance behavior.
    #[allow(
        clippy::too_many_lines,
        reason = "Complex parsing of raw locale data with multiple term types"
    )]
    pub(super) fn from_raw_with_base(raw: raw::RawLocale, base: Self) -> Self {
        let punctuation_in_quote = raw.locale.starts_with("en-US")
            || (raw.locale.starts_with("en") && !raw.locale.starts_with("en-GB"));

        let mut locale = base;
        locale.locale = raw.locale.clone();
        Self::remove_base_messages_shadowed_by_raw_terms(&raw, &mut locale.messages);
        locale.dates = DateTerms {
            months: MonthNames {
                long: raw.dates.months.long.into_map(SubYearCode::MIN_MONTH),
                short: raw.dates.months.short.into_map(SubYearCode::MIN_MONTH),
            },
            seasons: raw.dates.seasons.into_map(SubYearCode::MIN_SEASON),
            uncertainty_term: raw.dates.uncertainty_term,
            open_ended_term: raw.dates.open_ended_term,
            am: raw.dates.am,
            pm: raw.dates.pm,
            timezone_utc: raw.dates.timezone_utc,
            before_era: raw.dates.before_era,
            ad: raw.dates.ad,
            bc: raw.dates.bc,
            bce: raw.dates.bce,
            ce: raw.dates.ce,
        };
        locale.punctuation_in_quote = punctuation_in_quote;
        locale.sort_articles = Self::default_articles_for_locale(&raw.locale);

        locale.locale_schema_version = raw.locale_schema_version;
        locale.evaluation = raw.evaluation.unwrap_or_default();
        locale.messages.extend(raw.messages);
        locale.date_formats.extend(raw.date_formats);
        locale.legacy_term_aliases.extend(raw.legacy_term_aliases);

        if let Some(raw_vocab) = raw.vocab {
            locale.vocab.genre.extend(raw_vocab.genre);
            locale.vocab.medium.extend(raw_vocab.medium);
        }

        if let Some(go) = raw.grammar_options {
            locale.grammar_options = go;
        } else {
            locale.grammar_options.punctuation_in_quote = locale.punctuation_in_quote;
        }
        locale.punctuation_in_quote = locale.grammar_options.punctuation_in_quote;
        locale.punctuation_realization = raw.punctuation_realization;

        if let Some(nf) = raw.number_formats {
            locale.number_formats = nf;
        }

        let explicit_locator_keys: std::collections::HashSet<LocatorType> = raw
            .locators
            .keys()
            .filter_map(|key| Self::parse_builtin_locator_type(key))
            .collect();

        for (key, value) in &raw.locators {
            if let Some(locator_type) = Self::parse_locator_type(key) {
                let locator_term = LocatorTerm {
                    long: Self::extract_singular_plural(value.long.as_ref().as_ref()),
                    short: Self::extract_singular_plural(value.short.as_ref().as_ref()),
                    symbol: Self::extract_singular_plural(value.symbol.as_ref().as_ref()),
                    gender: value.gender.clone(),
                };
                locale.locators.insert(locator_type, locator_term);
            }
        }

        for (key, value) in &raw.terms {
            if let Some(locator_type) = Self::parse_builtin_locator_type(key)
                && !explicit_locator_keys.contains(&locator_type)
                && let Some(forms) = Self::get_forms(value)
            {
                let locator_term = LocatorTerm {
                    long: Self::extract_singular_plural(forms.get("long").as_ref()),
                    short: Self::extract_singular_plural(forms.get("short").as_ref()),
                    symbol: Self::extract_singular_plural(forms.get("symbol").as_ref()),
                    gender: None,
                };
                locale.locators.insert(locator_type, locator_term);
                continue;
            }

            match key.as_str() {
                "and" => {
                    if let Some(forms) = Self::get_forms(value) {
                        if let Some(v) = forms.get("long").and_then(|v| v.as_string()) {
                            locale.terms.and = Some(v.to_string());
                        }
                        if let Some(v) = forms.get("symbol").and_then(|v| v.as_string()) {
                            locale.terms.and_symbol = Some(v.to_string());
                        }
                    }
                }
                "et_al" => {
                    if let Some(forms) = Self::get_forms(value)
                        && let Some(v) = forms.get("long").and_then(|v| v.as_string())
                    {
                        locale.terms.et_al = Some(v.to_string());
                    }
                }
                "and others" | "and_others" => {
                    if let Some(forms) = Self::get_forms(value)
                        && let Some(v) = forms.get("long").and_then(|v| v.as_string())
                    {
                        locale.terms.and_others = Some(v.to_string());
                    }
                }
                "accessed" => {
                    if let Some(forms) = Self::get_forms(value)
                        && let Some(v) = forms.get("long").and_then(|v| v.as_string())
                    {
                        locale.terms.accessed = Some(v.to_string());
                    }
                }
                "ibid" => {
                    if let Some(forms) = Self::get_forms(value)
                        && let Some(v) = forms.get("long").and_then(|v| v.as_string())
                    {
                        locale.terms.ibid = Some(v.to_string());
                    }
                }
                "no date" => {
                    let simple = Self::extract_simple_term_from_raw(value);
                    let short_fallback = simple.short.as_default_str().to_string();
                    locale
                        .terms
                        .general
                        .insert(super::types::GeneralTerm::NoDate, simple);
                    locale.terms.no_date.get_or_insert(short_fallback);
                }
                "no_date" => {
                    let simple = Self::extract_simple_term_from_raw(value);
                    locale.terms.no_date = Some(simple.short.as_str().to_string());
                    locale
                        .terms
                        .general
                        .entry(super::types::GeneralTerm::NoDate)
                        .or_insert(simple);
                }
                _ => {
                    if let Some(general_term) = Self::parse_general_term(key) {
                        let simple = Self::extract_simple_term_from_raw(value);
                        locale.terms.general.insert(general_term, simple);
                    } else {
                        let normalized = Self::normalize_term_key(key);
                        if Self::is_known_type_term_key(&normalized) {
                            let simple = Self::extract_simple_term_from_raw(value);
                            locale.type_terms.insert(normalized, simple);
                        }
                    }
                }
            }
        }

        for (key, role_term) in &raw.roles {
            let contributor_term = ContributorTerm {
                singular: Self::extract_simple_term(&role_term.long, &role_term.short, false),
                plural: Self::extract_simple_term(&role_term.long, &role_term.short, true),
                verb: Self::extract_verb_term(&role_term.verb, &role_term.verb_short),
            };
            if let Some(role) = Self::parse_role_name(key) {
                locale.roles.insert(role, contributor_term);
            } else {
                let canonical = if key == "editortranslator" {
                    "editor-translator".to_string()
                } else {
                    Self::normalize_term_key(key)
                };
                locale.role_combinations.insert(canonical, contributor_term);
            }
        }

        locale.evaluator = match locale.evaluation.message_syntax {
            MessageSyntax::Mf2 => Arc::new(Mf2MessageEvaluator) as Arc<dyn MessageEvaluator>,
            MessageSyntax::Static => Arc::new(NoOpEvaluator),
        };

        locale
    }

    /// Get default articles for a locale based on language code.
    fn default_articles_for_locale(locale_id: &str) -> Vec<String> {
        #[allow(clippy::string_slice, reason = "locale_id is expected to be ASCII")]
        let lang = &locale_id[..2.min(locale_id.len())];
        match lang {
            "en" => vec!["the".into(), "a".into(), "an".into()],
            "de" => vec![
                "der".into(),
                "die".into(),
                "das".into(),
                "ein".into(),
                "eine".into(),
            ],
            "fr" => vec![
                "le".into(),
                "la".into(),
                "les".into(),
                "l'".into(),
                "un".into(),
                "une".into(),
            ],
            "es" => vec![
                "el".into(),
                "la".into(),
                "los".into(),
                "las".into(),
                "un".into(),
                "una".into(),
            ],
            "it" => vec![
                "il".into(),
                "lo".into(),
                "la".into(),
                "i".into(),
                "gli".into(),
                "le".into(),
                "un".into(),
                "una".into(),
            ],
            "pt" => vec![
                "o".into(),
                "a".into(),
                "os".into(),
                "as".into(),
                "um".into(),
                "uma".into(),
            ],
            "nl" => vec!["de".into(), "het".into(), "een".into()],
            _ => vec![],
        }
    }

    fn get_forms(value: &raw::RawTermValue) -> Option<&HashMap<String, raw::RawTermValue>> {
        match value {
            raw::RawTermValue::Forms(forms) => Some(forms),
            _ => None,
        }
    }

    fn parse_locator_type(name: &str) -> Option<LocatorType> {
        LocatorType::from_key(name).ok()
    }

    fn parse_builtin_locator_type(name: &str) -> Option<LocatorType> {
        match Self::parse_locator_type(name)? {
            LocatorType::Custom(_) => None,
            locator => Some(locator),
        }
    }

    fn parse_role_name(name: &str) -> Option<ContributorRole> {
        match name {
            "author" => Some(ContributorRole::Author),
            "chair" => Some(ContributorRole::Chair),
            "editor" => Some(ContributorRole::Editor),
            "translator" => Some(ContributorRole::Translator),
            "annotator" => Some(ContributorRole::Annotator),
            "commentator" => Some(ContributorRole::Commentator),
            "foreword-author" => Some(ContributorRole::ForewordAuthor),
            "introduction-author" => Some(ContributorRole::IntroductionAuthor),
            "afterword-author" => Some(ContributorRole::AfterwordAuthor),
            "director" => Some(ContributorRole::Director),
            "compiler" => Some(ContributorRole::Composer),
            "illustrator" => Some(ContributorRole::Illustrator),
            "narrator" => Some(ContributorRole::Narrator),
            "collection-editor" => Some(ContributorRole::CollectionEditor),
            "container-author" => Some(ContributorRole::ContainerAuthor),
            "editorial-director" => Some(ContributorRole::EditorialDirector),
            "textual-editor" | "textual_editor" => Some(ContributorRole::TextualEditor),
            "interviewer" => Some(ContributorRole::Interviewer),
            "original-author" => Some(ContributorRole::OriginalAuthor),
            "recipient" => Some(ContributorRole::Recipient),
            "reviewed-author" => Some(ContributorRole::ReviewedAuthor),
            "performer" => Some(ContributorRole::Performer),
            "composer" => Some(ContributorRole::Composer),
            "writer" => Some(ContributorRole::Writer),
            "producer" => Some(ContributorRole::Producer),
            _ => None,
        }
    }

    fn remove_base_messages_shadowed_by_raw_terms(
        raw: &raw::RawLocale,
        messages: &mut HashMap<String, String>,
    ) {
        for key in raw.locators.keys() {
            if let Some(locator) = Self::parse_builtin_locator_type(key) {
                for form in [TermForm::Long, TermForm::Short] {
                    if let Some(message_id) = Self::locator_message_id(&locator, &form) {
                        messages.remove(message_id);
                    }
                }
            }
        }

        for key in raw.roles.keys() {
            if let Some(role) = Self::parse_role_name(key) {
                for form in [
                    TermForm::Long,
                    TermForm::Short,
                    TermForm::Verb,
                    TermForm::VerbShort,
                ] {
                    if let Some(message_id) = Self::role_message_id(&role, &form) {
                        messages.remove(message_id);
                    }
                }
            }
        }

        for key in raw.terms.keys() {
            if let Some(term) = Self::parse_general_term(key) {
                for form in [TermForm::Long, TermForm::Short] {
                    if let Some(message_id) = Self::general_message_id(&term, &form) {
                        messages.remove(message_id);
                    }
                }
            }
        }
    }

    fn extract_singular_plural(value: Option<&&raw::RawTermValue>) -> Option<SingularPlural> {
        match value {
            Some(raw::RawTermValue::SingularPlural { singular, plural }) => Some(SingularPlural {
                singular: Self::from_raw_gendered_string(singular),
                plural: Self::from_raw_gendered_string(plural),
            }),
            Some(raw::RawTermValue::Simple(s)) => Some(SingularPlural {
                singular: MaybeGendered::Plain(s.clone()),
                plural: MaybeGendered::Plain(s.clone()),
            }),
            Some(raw::RawTermValue::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            }) => Some(SingularPlural {
                singular: MaybeGendered::Gendered {
                    masculine: masculine.clone(),
                    feminine: feminine.clone(),
                    neuter: neuter.clone(),
                    common: common.clone(),
                },
                plural: MaybeGendered::Gendered {
                    masculine: masculine.clone(),
                    feminine: feminine.clone(),
                    neuter: neuter.clone(),
                    common: common.clone(),
                },
            }),
            Some(raw::RawTermValue::Forms(forms)) => {
                let singular = forms
                    .get("singular")
                    .map(Self::extract_maybe_gendered_string);
                let plural = forms.get("plural").map(Self::extract_maybe_gendered_string);

                singular.map(|s| SingularPlural {
                    plural: plural.unwrap_or_else(|| s.clone()),
                    singular: s,
                })
            }
            _ => None,
        }
    }

    fn extract_simple_term(
        long: &Option<raw::RawTermValue>,
        short: &Option<raw::RawTermValue>,
        plural: bool,
    ) -> SimpleTerm {
        let long_str = long
            .as_ref()
            .map(|v| Self::extract_simple_gendered_term(v, plural))
            .unwrap_or_default();

        let short_str = short
            .as_ref()
            .map(|v| Self::extract_simple_gendered_term(v, plural))
            .unwrap_or_default();

        SimpleTerm {
            long: long_str,
            short: short_str,
        }
    }

    fn extract_verb_term(
        verb: &Option<raw::RawTermValue>,
        verb_short: &Option<raw::RawTermValue>,
    ) -> SimpleTerm {
        let long_str = verb
            .as_ref()
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .into();

        let short_str = verb_short
            .as_ref()
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .into();

        SimpleTerm {
            long: long_str,
            short: short_str,
        }
    }

    /// Normalize a locale term key to canonical kebab-case.
    ///
    /// Locale YAML files and style templates may use underscores or spaces
    /// interchangeably with hyphens (e.g. `no_date`, `no date`, `no-date`).
    /// This helper converts all three forms to the single canonical
    /// kebab-case key so `parse_general_term` only needs to match one pattern
    /// per term.
    fn normalize_term_key(s: &str) -> String {
        s.replace(['_', ' '], "-")
    }

    /// Parse a locale term key into a structured general-term identifier.
    pub fn parse_general_term(name: &str) -> Option<super::types::GeneralTerm> {
        use super::types::GeneralTerm;
        match Self::normalize_term_key(name).as_str() {
            "in" => Some(GeneralTerm::In),
            "accessed" => Some(GeneralTerm::Accessed),
            "cited" => Some(GeneralTerm::Cited),
            "retrieved" => Some(GeneralTerm::Retrieved),
            "at" => Some(GeneralTerm::At),
            "from" => Some(GeneralTerm::From),
            "of" => Some(GeneralTerm::Of),
            "to" => Some(GeneralTerm::To),
            "by" => Some(GeneralTerm::By),
            "no-date" => Some(GeneralTerm::NoDate),
            "anonymous" => Some(GeneralTerm::Anonymous),
            "circa" => Some(GeneralTerm::Circa),
            "available-at" => Some(GeneralTerm::AvailableAt),
            "ibid" => Some(GeneralTerm::Ibid),
            "and" => Some(GeneralTerm::And),
            "role-conjunction" => Some(GeneralTerm::RoleConjunction),
            "et-al" => Some(GeneralTerm::EtAl),
            "and-others" => Some(GeneralTerm::AndOthers),
            "forthcoming" => Some(GeneralTerm::Forthcoming),
            "online" => Some(GeneralTerm::Online),
            "here" => Some(GeneralTerm::Here),
            "deposited" => Some(GeneralTerm::Deposited),
            "review-of" => Some(GeneralTerm::ReviewOf),
            "original-work-published" => Some(GeneralTerm::OriginalWorkPublished),
            "personal-communication" => Some(GeneralTerm::PersonalCommunication),
            "patent" => Some(GeneralTerm::Patent),
            "issued" => Some(GeneralTerm::Issued),
            "volume" => Some(GeneralTerm::Volume),
            "issue" => Some(GeneralTerm::Issue),
            "page" => Some(GeneralTerm::Page),
            "chapter" => Some(GeneralTerm::Chapter),
            "edition" => Some(GeneralTerm::Edition),
            "section" => Some(GeneralTerm::Section),
            "version" => Some(GeneralTerm::Version),
            _ => None,
        }
    }

    /// Reference-type description term keys recognized for
    /// [`Locale::type_terms`] (the `type-label` component's fallback data).
    ///
    /// Legacy locale files (`terms:`) carry a wide mix of CSL 1.0 term keys
    /// — general terms, locator labels, and reference-type descriptions —
    /// under one flat map. General terms and locators are already claimed by
    /// `parse_general_term` and `parse_builtin_locator_type` above; this is
    /// an explicit allowlist of the *remaining* keys that are genuinely
    /// reference-type descriptions, cross-referenced against the finite set
    /// of strings `citum_schema_data`'s `Reference::ref_type()` can actually
    /// produce (see `citum-engine/src/values/type_class.rs` module docs).
    ///
    /// This is deliberately an explicit list, not "capture anything
    /// unclaimed" — the unclaimed remainder also includes unrelated dead
    /// data (era terms, punctuation terms, number-variable labels like
    /// `version`/`printing`) that must not leak into `type_terms`.
    ///
    /// Membership here only makes `en-US.yaml`'s existing term *reachable*;
    /// it does not guarantee every `ref_type()` output has a term to reach.
    /// A handful of real `ref_type()` outputs currently have no authored
    /// term in `en-US.yaml` at all — `book`, `brief`, `bill-proceeding`,
    /// `bill-record`, `chapter`, `figure`, `manual`, `statute` — so
    /// `type-label` falls back to an empty label for those types today.
    /// That is a locale-content gap (nothing to allowlist), not a bug in
    /// this function; only `dataset` is exercised by a shipped style so far.
    fn is_known_type_term_key(normalized_key: &str) -> bool {
        const KNOWN_TYPE_TERM_KEYS: &[&str] = &[
            "article-journal",
            "article-magazine",
            "article-newspaper",
            "broadcast",
            "classic",
            "collection",
            "dataset",
            "document",
            "entry",
            "entry-dictionary",
            "entry-encyclopedia",
            "event",
            "graphic",
            "hearing",
            "interview",
            "legal-case",
            "legislation",
            "manuscript",
            "map",
            "motion-picture",
            "musical-score",
            "pamphlet",
            "paper-conference",
            "performance",
            "periodical",
            "personal-communication",
            "post",
            "post-weblog",
            "preprint",
            "regulation",
            "report",
            "review",
            "review-book",
            "software",
            "song",
            "speech",
            "standard",
            "thesis",
            "treaty",
            "webpage",
        ];
        KNOWN_TYPE_TERM_KEYS.contains(&normalized_key)
    }

    fn extract_simple_term_from_raw(value: &raw::RawTermValue) -> SimpleTerm {
        match value {
            raw::RawTermValue::Simple(s) => SimpleTerm {
                long: s.clone().into(),
                short: s.clone().into(),
            },
            raw::RawTermValue::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            } => SimpleTerm {
                long: MaybeGendered::Gendered {
                    masculine: masculine.clone(),
                    feminine: feminine.clone(),
                    neuter: neuter.clone(),
                    common: common.clone(),
                },
                short: MaybeGendered::Gendered {
                    masculine: masculine.clone(),
                    feminine: feminine.clone(),
                    neuter: neuter.clone(),
                    common: common.clone(),
                },
            },
            raw::RawTermValue::Forms(forms) => {
                let long = forms
                    .get("long")
                    .map(Self::extract_maybe_gendered_string)
                    .unwrap_or_default();
                let short = forms
                    .get("short")
                    .map(Self::extract_maybe_gendered_string)
                    .unwrap_or_else(|| long.clone());
                SimpleTerm { long, short }
            }
            raw::RawTermValue::SingularPlural { singular, .. } => SimpleTerm {
                long: Self::from_raw_gendered_string(singular),
                short: Self::from_raw_gendered_string(singular),
            },
        }
    }

    fn from_raw_gendered_string(value: &raw::RawGenderedString) -> MaybeGendered<String> {
        match value {
            raw::RawGenderedString::Simple(value) => MaybeGendered::Plain(value.clone()),
            raw::RawGenderedString::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            } => MaybeGendered::Gendered {
                masculine: masculine.clone(),
                feminine: feminine.clone(),
                neuter: neuter.clone(),
                common: common.clone(),
            },
        }
    }

    fn extract_maybe_gendered_string(value: &raw::RawTermValue) -> MaybeGendered<String> {
        match value {
            raw::RawTermValue::Simple(value) => MaybeGendered::Plain(value.clone()),
            raw::RawTermValue::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            } => MaybeGendered::Gendered {
                masculine: masculine.clone(),
                feminine: feminine.clone(),
                neuter: neuter.clone(),
                common: common.clone(),
            },
            raw::RawTermValue::SingularPlural { singular, .. } => {
                Self::from_raw_gendered_string(singular)
            }
            raw::RawTermValue::Forms(forms) => forms
                .get("long")
                .or_else(|| forms.get("singular"))
                .map(Self::extract_maybe_gendered_string)
                .unwrap_or_default(),
        }
    }

    fn extract_simple_gendered_term(
        value: &raw::RawTermValue,
        plural: bool,
    ) -> MaybeGendered<String> {
        match value {
            raw::RawTermValue::Simple(value) => MaybeGendered::Plain(value.clone()),
            raw::RawTermValue::Gendered {
                masculine,
                feminine,
                neuter,
                common,
            } => MaybeGendered::Gendered {
                masculine: masculine.clone(),
                feminine: feminine.clone(),
                neuter: neuter.clone(),
                common: common.clone(),
            },
            raw::RawTermValue::SingularPlural {
                singular,
                plural: plural_value,
            } => {
                if plural {
                    Self::from_raw_gendered_string(plural_value)
                } else {
                    Self::from_raw_gendered_string(singular)
                }
            }
            raw::RawTermValue::Forms(forms) => {
                let key = if plural { "plural" } else { "singular" };
                forms
                    .get(key)
                    .or_else(|| forms.get("long"))
                    .map(Self::extract_maybe_gendered_string)
                    .unwrap_or_default()
            }
        }
    }

    /// Apply a partial override, merging its fields into this locale.
    ///
    /// Performs key-by-key insertion or replacement for:
    /// - `messages`: new or updated message IDs
    /// - `grammar_options`: if `Some`, replaces the entire block and syncs
    ///   `punctuation_in_quote` field
    /// - `legacy_term_aliases`: new or updated term aliases
    /// - `dates`: new or updated month/season names, by EDTF sub-year code
    pub fn apply_override(&mut self, ov: &LocaleOverride) {
        for (k, v) in &ov.messages {
            self.messages.insert(k.clone(), v.clone());
        }
        if let Some(go) = &ov.grammar_options {
            self.grammar_options = go.clone();
            self.punctuation_in_quote = go.punctuation_in_quote;
        }
        for (k, v) in &ov.legacy_term_aliases {
            self.legacy_term_aliases.insert(k.clone(), v.clone());
        }
        for (k, v) in &ov.dates.months.long {
            self.dates.months.long.insert(*k, v.clone());
        }
        for (k, v) in &ov.dates.months.short {
            self.dates.months.short.insert(*k, v.clone());
        }
        for (k, v) in &ov.dates.seasons {
            self.dates.seasons.insert(*k, v.clone());
        }
    }
}
