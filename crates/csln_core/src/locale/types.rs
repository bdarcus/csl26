/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Form for term lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TermForm {
    Long,
    Short,
    Verb,
    VerbShort,
    Symbol,
}

/// A list of general terms for citation formatting.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GeneralTerm {
    #[default]
    In,
    Accessed,
    Retrieved,
    At,
    From,
    By,
    NoDate,
    Anonymous,
    Circa,
    AvailableAt,
    Ibid,
    And,
    EtAl,
    AndOthers,
    Forthcoming,
    Online,
    ReviewOf,
    OriginalWorkPublished,
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
    /// All other general terms.
    #[serde(flatten, default)]
    pub general: std::collections::HashMap<GeneralTerm, SimpleTerm>,
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
            general: std::collections::HashMap::new(),
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
    /// Term for uncertain dates (e.g., "uncertain").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncertainty_term: Option<String>,
    /// Term for open-ended date ranges (e.g., "present").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_ended_term: Option<String>,
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
            uncertainty_term: Some("uncertain".into()),
            open_ended_term: Some("present".into()),
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
