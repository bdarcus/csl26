/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Locale definitions for CSLN.
//!
//! Locales provide language-specific terms, date formats, and punctuation rules
//! for citation formatting.

use crate::citation::LocatorType;
use crate::template::ContributorRole;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A list of month names (12 elements for Jan-Dec).
pub type MonthList = Vec<String>;

/// A locale definition containing language-specific terms and formatting rules.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Locale {
    /// The locale identifier (e.g., "en-US", "de-DE").
    pub locale: String,
    /// Date-related terms (months, seasons).
    #[serde(default)]
    pub dates: DateTerms,
    /// Contributor role terms (editor, translator, etc.).
    #[serde(default)]
    pub roles: HashMap<ContributorRole, ContributorTerm>,
    /// Locator terms (page, chapter, etc.).
    #[serde(default)]
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
}

impl Locale {
    /// Create a new English (US) locale with default terms.
    pub fn en_us() -> Self {
        let mut roles = HashMap::new();

        roles.insert(
            ContributorRole::Editor,
            ContributorTerm {
                singular: SimpleTerm {
                    long: "editor".into(),
                    short: "Ed.".into(),
                },
                plural: SimpleTerm {
                    long: "editors".into(),
                    short: "Eds.".into(),
                },
                verb: SimpleTerm {
                    long: "edited by".into(),
                    short: "Ed.".into(),
                },
            },
        );

        roles.insert(
            ContributorRole::Translator,
            ContributorTerm {
                singular: SimpleTerm {
                    long: "translator".into(),
                    short: "Trans.".into(),
                },
                plural: SimpleTerm {
                    long: "translators".into(),
                    short: "Trans.".into(),
                },
                verb: SimpleTerm {
                    long: "translated by".into(),
                    short: "Trans.".into(),
                },
            },
        );

        roles.insert(
            ContributorRole::Director,
            ContributorTerm {
                singular: SimpleTerm {
                    long: "director".into(),
                    short: "Dir.".into(),
                },
                plural: SimpleTerm {
                    long: "directors".into(),
                    short: "dirs.".into(),
                },
                verb: SimpleTerm {
                    long: "directed by".into(),
                    short: "dir.".into(),
                },
            },
        );

        Self {
            locale: "en-US".into(),
            dates: DateTerms::en_us(),
            roles,
            locators: HashMap::new(), // Populated in from_raw usually
            terms: Terms::en_us(),
            punctuation_in_quote: true, // American English convention
            sort_articles: vec!["the".into(), "a".into(), "an".into()],
        }
    }

    /// Strip leading articles from a string for sorting.
    ///
    /// Uses locale-specific articles (e.g., "the", "a", "an" for English;
    /// "der", "die", "das" for German). Falls back to English articles
    /// if no locale-specific articles are defined.
    pub fn strip_sort_articles<'a>(&self, s: &'a str) -> &'a str {
        let s = s.trim();

        // Default English articles
        const DEFAULT_ARTICLES: &[&str] = &["the", "a", "an"];

        if self.sort_articles.is_empty() {
            // Use default English articles
            for article in DEFAULT_ARTICLES {
                let prefix = format!("{} ", article);
                if s.to_lowercase().starts_with(&prefix) {
                    return &s[prefix.len()..];
                }
            }
        } else {
            // Use locale-specific articles
            for article in &self.sort_articles {
                let prefix = format!("{} ", article);
                if s.to_lowercase().starts_with(&prefix) {
                    return &s[prefix.len()..];
                }
            }
        }
        s
    }

    /// Get default articles for a locale based on language code.
    fn default_articles_for_locale(locale_id: &str) -> Vec<String> {
        // Extract language code (first 2 chars)
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
            _ => vec![], // Fall back to English defaults in strip_sort_articles
        }
    }

    /// Get a contributor role term.
    pub fn role_term(&self, role: &ContributorRole, plural: bool, form: TermForm) -> Option<&str> {
        let term = self.roles.get(role)?;
        let simple = if plural { &term.plural } else { &term.singular };
        Some(match form {
            TermForm::Long => &simple.long,
            TermForm::Short => &simple.short,
            TermForm::Verb => &term.verb.long,
            TermForm::VerbShort => &term.verb.short,
            _ => &simple.long, // Fallback
        })
    }

    /// Get a locator term.
    pub fn locator_term(
        &self,
        locator: &LocatorType,
        plural: bool,
        form: TermForm,
    ) -> Option<&str> {
        let term = self.locators.get(locator)?;
        let form_term = match form {
            TermForm::Long => &term.long,
            TermForm::Short => &term.short,
            TermForm::Symbol => &term.symbol,
            _ => &term.short, // Fallback
        };

        if let Some(ft) = form_term {
            Some(if plural { &ft.plural } else { &ft.singular })
        } else {
            None
        }
    }

    /// Get the "and" term based on style preference.
    pub fn and_term(&self, use_symbol: bool) -> &str {
        if use_symbol {
            self.terms.and_symbol.as_deref().unwrap_or("&")
        } else {
            self.terms.and.as_deref().unwrap_or("and")
        }
    }

    /// Get the "et al." term.
    pub fn et_al(&self) -> &str {
        self.terms.et_al.as_deref().unwrap_or("et al.")
    }

    /// Get a month name.
    pub fn month_name(&self, month: u8, short: bool) -> &str {
        let idx = (month.saturating_sub(1)) as usize;
        if short {
            self.dates
                .months
                .short
                .get(idx)
                .map(|s| s.as_str())
                .unwrap_or("")
        } else {
            self.dates
                .months
                .long
                .get(idx)
                .map(|s| s.as_str())
                .unwrap_or("")
        }
    }
}

/// Form for term lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum TermForm {
    Long,
    Short,
    Verb,
    VerbShort,
    Symbol,
}

/// General terms used in citations and bibliographies.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Terms {
    /// The word "and" (e.g., "Smith and Jones").
    pub and: Option<String>,
    /// Symbol form of "and" (e.g., "&").
    pub and_symbol: Option<String>,
    /// "and others" for generic use.
    pub and_others: Option<String>,
    /// Anonymous author term.
    #[serde(default)]
    pub anonymous: SimpleTerm,
    /// "at" preposition.
    pub at: Option<String>,
    /// "accessed" for URLs.
    pub accessed: Option<String>,
    /// "available at" for URLs.
    pub available_at: Option<String>,
    /// "by" preposition.
    pub by: Option<String>,
    /// "circa" for approximate dates.
    #[serde(default)]
    pub circa: SimpleTerm,
    /// "et al." abbreviation.
    pub et_al: Option<String>,
    /// "from" preposition.
    pub from: Option<String>,
    /// "ibid." for repeated citations.
    pub ibid: Option<String>,
    /// "in" preposition.
    pub in_: Option<String>,
    /// "no date" for missing dates.
    pub no_date: Option<String>,
    /// "retrieved" for access dates.
    pub retrieved: Option<String>,
}

impl Terms {
    /// Create English (US) terms.
    pub fn en_us() -> Self {
        Self {
            and: Some("and".into()),
            and_symbol: Some("&".into()),
            and_others: Some("and others".into()),
            anonymous: SimpleTerm {
                long: "anonymous".into(),
                short: "anon.".into(),
            },
            at: Some("at".into()),
            accessed: Some("accessed".into()),
            available_at: Some("available at".into()),
            by: Some("by".into()),
            circa: SimpleTerm {
                long: "circa".into(),
                short: "c.".into(),
            },
            et_al: Some("et al.".into()),
            from: Some("from".into()),
            ibid: Some("ibid.".into()),
            in_: Some("in".into()),
            no_date: Some("n.d.".into()),
            retrieved: Some("retrieved".into()),
        }
    }
}

/// A simple term with long and short forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct SimpleTerm {
    /// The long form of the term.
    pub long: String,
    /// The short form of the term.
    pub short: String,
}

/// Terms for contributor roles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct ContributorTerm {
    /// Singular form (editor, translator).
    pub singular: SimpleTerm,
    /// Plural form (editors, translators).
    pub plural: SimpleTerm,
    /// Verb form (edited by, translated by).
    pub verb: SimpleTerm,
}

/// Terms for locators (page, chapter, etc.).
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct LocatorTerm {
    /// Long form (e.g., page/pages).
    #[serde(default)]
    pub long: Option<SingularPlural>,
    /// Short form (e.g., p./pp.).
    #[serde(default)]
    pub short: Option<SingularPlural>,
    /// Symbol form (e.g., §/§§).
    #[serde(default)]
    pub symbol: Option<SingularPlural>,
}

/// A term with singular and plural forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct SingularPlural {
    /// Singular form.
    pub singular: String,
    /// Plural form.
    pub plural: String,
}

/// Date-related terms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct DateTerms {
    /// Month names.
    #[serde(default)]
    pub months: MonthNames,
    /// Season names (Spring, Summer, Autumn, Winter).
    #[serde(default)]
    pub seasons: Vec<String>,
}

impl DateTerms {
    /// Create English (US) date terms.
    pub fn en_us() -> Self {
        Self {
            months: MonthNames::en_us(),
            seasons: vec![
                "Spring".into(),
                "Summer".into(),
                "Autumn".into(),
                "Winter".into(),
            ],
        }
    }
}

/// Month name lists.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct MonthNames {
    /// Full month names.
    pub long: Vec<String>,
    /// Abbreviated month names.
    pub short: Vec<String>,
}

impl MonthNames {
    /// Create English month names.
    pub fn en_us() -> Self {
        Self {
            long: vec![
                "January".into(),
                "February".into(),
                "March".into(),
                "April".into(),
                "May".into(),
                "June".into(),
                "July".into(),
                "August".into(),
                "September".into(),
                "October".into(),
                "November".into(),
                "December".into(),
            ],
            short: vec![
                "Jan.".into(),
                "Feb.".into(),
                "Mar.".into(),
                "Apr.".into(),
                "May".into(),
                "June".into(),
                "July".into(),
                "Aug.".into(),
                "Sept.".into(),
                "Oct.".into(),
                "Nov.".into(),
                "Dec.".into(),
            ],
        }
    }
}

/// Raw locale format for YAML parsing.
/// This is a simpler format that uses string keys for terms.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
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
}

/// Raw date terms for YAML parsing.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct RawDateTerms {
    #[serde(default)]
    pub months: RawMonthNames,
    #[serde(default)]
    pub seasons: Vec<String>,
}

/// Raw month names for YAML parsing.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct RawMonthNames {
    #[serde(default)]
    pub long: Vec<String>,
    #[serde(default)]
    pub short: Vec<String>,
}

/// Raw role term with form-keyed values.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct RawRoleTerm {
    #[serde(default)]
    pub long: Option<RawTermValue>,
    #[serde(default)]
    pub short: Option<RawTermValue>,
    #[serde(default)]
    pub verb: Option<RawTermValue>,
    #[serde(default, rename = "verb-short")]
    pub verb_short: Option<RawTermValue>,
}

/// A term value that can be a simple string or have singular/plural forms.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RawTermValue {
    /// Simple string value.
    Simple(String),
    /// Form-keyed value (for terms with long/short forms).
    Forms(HashMap<String, RawTermValue>),
    /// Singular/plural forms.
    SingularPlural { singular: String, plural: String },
}

impl Default for RawTermValue {
    fn default() -> Self {
        RawTermValue::Simple(String::new())
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
}

impl Locale {
    /// Load a locale from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, String> {
        let raw: RawLocale = serde_yaml::from_str(yaml)
            .map_err(|e| format!("Failed to parse locale YAML: {}", e))?;

        Ok(Self::from_raw(raw))
    }

    /// Load locale from a file path directly.
    pub fn from_yaml_file(path: &std::path::Path) -> Result<Self, String> {
        let yaml = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read locale file: {}", e))?;
        Self::from_yaml_str(&yaml)
    }

    /// Load a locale by ID (e.g., "en-US", "de-DE") from a locales directory.
    /// Falls back to en-US if the locale file is not found.
    pub fn load(locale_id: &str, locales_dir: &std::path::Path) -> Self {
        let file_name = format!("{}.yaml", locale_id);
        let file_path = locales_dir.join(&file_name);

        if file_path.exists() {
            match Self::from_yaml_file(&file_path) {
                Ok(locale) => return locale,
                Err(e) => {
                    eprintln!("Warning: Failed to load locale {}: {}", locale_id, e);
                }
            }
        }

        // Try fallback to base locale (e.g., "de" from "de-DE")
        if locale_id.contains('-') {
            let base = locale_id.split('-').next().unwrap_or("en");
            // Try all files that start with base
            if let Ok(entries) = std::fs::read_dir(locales_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with(base) && name_str.ends_with(".yaml") {
                        if let Ok(locale) = Self::from_yaml_file(&entry.path()) {
                            return locale;
                        }
                    }
                }
            }
        }

        // Default to hardcoded en-US
        Self::en_us()
    }

    /// Convert a RawLocale to a Locale.
    fn from_raw(raw: RawLocale) -> Self {
        // Determine punctuation-in-quote from locale ID
        // en-US uses American style (inside), en-GB and others use outside
        let punctuation_in_quote = raw.locale.starts_with("en-US")
            || (raw.locale.starts_with("en") && !raw.locale.starts_with("en-GB"));

        let mut locale = Locale {
            locale: raw.locale.clone(),
            dates: DateTerms {
                months: MonthNames {
                    long: raw.dates.months.long,
                    short: raw.dates.months.short,
                },
                seasons: raw.dates.seasons,
            },
            roles: HashMap::new(),
            locators: HashMap::new(),
            terms: Terms::default(),
            punctuation_in_quote,
            // Set locale-specific articles based on language
            sort_articles: Self::default_articles_for_locale(&raw.locale),
        };

        // Map raw terms to structured terms and locators
        for (key, value) in &raw.terms {
            // First try to parse as a locator
            if let Some(locator_type) = Self::parse_locator_type(key) {
                if let Some(forms) = Self::get_forms(value) {
                    let locator_term = LocatorTerm {
                        long: Self::extract_singular_plural(&forms.get("long")),
                        short: Self::extract_singular_plural(&forms.get("short")),
                        symbol: Self::extract_singular_plural(&forms.get("symbol")),
                    };
                    locale.locators.insert(locator_type, locator_term);
                }
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
                    if let Some(forms) = Self::get_forms(value) {
                        if let Some(v) = forms.get("long").and_then(|v| v.as_string()) {
                            locale.terms.et_al = Some(v.to_string());
                        }
                    }
                }
                "and others" | "and_others" => {
                    if let Some(forms) = Self::get_forms(value) {
                        if let Some(v) = forms.get("long").and_then(|v| v.as_string()) {
                            locale.terms.and_others = Some(v.to_string());
                        }
                    }
                }
                "accessed" => {
                    if let Some(forms) = Self::get_forms(value) {
                        if let Some(v) = forms.get("long").and_then(|v| v.as_string()) {
                            locale.terms.accessed = Some(v.to_string());
                        }
                    }
                }
                "ibid" => {
                    if let Some(forms) = Self::get_forms(value) {
                        if let Some(v) = forms.get("long").and_then(|v| v.as_string()) {
                            locale.terms.ibid = Some(v.to_string());
                        }
                    }
                }
                "no_date" | "no date" => {
                    if let Some(forms) = Self::get_forms(value) {
                        if let Some(v) = forms.get("short").and_then(|v| v.as_string()) {
                            locale.terms.no_date = Some(v.to_string());
                        } else if let Some(v) = forms.get("long").and_then(|v| v.as_string()) {
                            locale.terms.no_date = Some(v.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        // Map raw roles to structured roles (simplified for now)
        for (key, role_term) in &raw.roles {
            if let Some(role) = Self::parse_role_name(key) {
                let contributor_term = ContributorTerm {
                    singular: Self::extract_simple_term(&role_term.long, &role_term.short, false),
                    plural: Self::extract_simple_term(&role_term.long, &role_term.short, true),
                    verb: Self::extract_verb_term(&role_term.verb, &role_term.verb_short),
                };
                locale.roles.insert(role, contributor_term);
            }
        }

        locale
    }

    fn get_forms(value: &RawTermValue) -> Option<&HashMap<String, RawTermValue>> {
        match value {
            RawTermValue::Forms(forms) => Some(forms),
            _ => None,
        }
    }

    fn parse_locator_type(name: &str) -> Option<LocatorType> {
        match name {
            "book" => Some(LocatorType::Book),
            "chapter" => Some(LocatorType::Chapter),
            "column" => Some(LocatorType::Column),
            "figure" => Some(LocatorType::Figure),
            "folio" => Some(LocatorType::Folio),
            "line" => Some(LocatorType::Line),
            "note" => Some(LocatorType::Note),
            "number" => Some(LocatorType::Number),
            "opus" => Some(LocatorType::Opus),
            "page" => Some(LocatorType::Page),
            "paragraph" => Some(LocatorType::Paragraph),
            "part" => Some(LocatorType::Part),
            "section" => Some(LocatorType::Section),
            "sub_verbo" | "sub-verbo" => Some(LocatorType::SubVerbo),
            "verse" => Some(LocatorType::Verse),
            "volume" => Some(LocatorType::Volume),
            "issue" => Some(LocatorType::Issue),
            _ => None,
        }
    }

    fn parse_role_name(name: &str) -> Option<ContributorRole> {
        match name {
            "author" => Some(ContributorRole::Author),
            "editor" => Some(ContributorRole::Editor),
            "translator" => Some(ContributorRole::Translator),
            "director" => Some(ContributorRole::Director),
            "compiler" => Some(ContributorRole::Composer), // Close mapping
            "illustrator" => Some(ContributorRole::Illustrator),
            "collection-editor" => Some(ContributorRole::CollectionEditor),
            "container-author" => Some(ContributorRole::ContainerAuthor),
            "editorial-director" => Some(ContributorRole::EditorialDirector),
            "interviewer" => Some(ContributorRole::Interviewer),
            "original-author" => Some(ContributorRole::OriginalAuthor),
            "recipient" => Some(ContributorRole::Recipient),
            "reviewed-author" => Some(ContributorRole::ReviewedAuthor),
            "composer" => Some(ContributorRole::Composer),
            _ => None,
        }
    }

    fn extract_singular_plural(value: &Option<&RawTermValue>) -> Option<SingularPlural> {
        match value {
            Some(RawTermValue::SingularPlural { singular, plural }) => Some(SingularPlural {
                singular: singular.clone(),
                plural: plural.clone(),
            }),
            Some(RawTermValue::Simple(s)) => Some(SingularPlural {
                singular: s.clone(),
                plural: s.clone(), // Fallback if only one form provided
            }),
            _ => None,
        }
    }

    fn extract_simple_term(
        long: &Option<RawTermValue>,
        short: &Option<RawTermValue>,
        plural: bool,
    ) -> SimpleTerm {
        let long_str = long
            .as_ref()
            .and_then(|v| match v {
                RawTermValue::Simple(s) => Some(s.clone()),
                RawTermValue::SingularPlural {
                    singular,
                    plural: p,
                } => Some(if plural { p.clone() } else { singular.clone() }),
                _ => None,
            })
            .unwrap_or_default();

        let short_str = short
            .as_ref()
            .and_then(|v| match v {
                RawTermValue::Simple(s) => Some(s.clone()),
                RawTermValue::SingularPlural {
                    singular,
                    plural: p,
                } => Some(if plural { p.clone() } else { singular.clone() }),
                _ => None,
            })
            .unwrap_or_default();

        SimpleTerm {
            long: long_str,
            short: short_str,
        }
    }

    fn extract_verb_term(
        verb: &Option<RawTermValue>,
        verb_short: &Option<RawTermValue>,
    ) -> SimpleTerm {
        let long_str = verb
            .as_ref()
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();

        let short_str = verb_short
            .as_ref()
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();

        SimpleTerm {
            long: long_str,
            short: short_str,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_en_us_locale() {
        let locale = Locale::en_us();
        assert_eq!(locale.locale, "en-US");
        assert_eq!(locale.and_term(false), "and");
        assert_eq!(locale.and_term(true), "&");
        assert_eq!(locale.et_al(), "et al.");
    }

    #[test]
    fn test_month_names() {
        let locale = Locale::en_us();
        assert_eq!(locale.month_name(1, false), "January");
        assert_eq!(locale.month_name(1, true), "Jan.");
        assert_eq!(locale.month_name(12, false), "December");
    }

    #[test]
    fn test_role_terms() {
        let locale = Locale::en_us();

        assert_eq!(
            locale.role_term(&ContributorRole::Editor, false, TermForm::Short),
            Some("Ed.")
        );
        assert_eq!(
            locale.role_term(&ContributorRole::Editor, true, TermForm::Short),
            Some("Eds.")
        );
        assert_eq!(
            locale.role_term(&ContributorRole::Translator, false, TermForm::Verb),
            Some("translated by")
        );
    }

    #[test]
    fn test_locale_deserialization() {
        let json = r#"{
            "locale": "en-US",
            "dates": {
                "months": {
                    "long": ["January", "February", "March", "April", "May", "June",
                             "July", "August", "September", "October", "November", "December"],
                    "short": ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                              "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
                },
                "seasons": ["Spring", "Summer", "Autumn", "Winter"]
            },
            "roles": {},
            "terms": {
                "and": "and",
                "et-al": "et al."
            }
        }"#;

        let locale: Locale = serde_json::from_str(json).unwrap();
        assert_eq!(locale.locale, "en-US");
        assert_eq!(locale.dates.months.long[0], "January");
        assert_eq!(locale.terms.and.as_ref().unwrap(), "and");
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
        assert_eq!(locale.and_term(false), "und");
        assert_eq!(locale.et_al(), "u. a.");
        assert_eq!(locale.month_name(1, false), "Januar");
        assert_eq!(locale.month_name(3, false), "März");
    }
}
