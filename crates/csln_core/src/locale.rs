/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Locale definitions for CSLN.
//!
//! Locales provide language-specific terms, date formats, and punctuation rules
//! for citation formatting.

use crate::template::ContributorRole;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// General terms (and, et al., etc.).
    #[serde(default)]
    pub terms: Terms,
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
            terms: Terms::en_us(),
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
        })
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
}
