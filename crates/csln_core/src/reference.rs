/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! A reference is a bibliographic item, such as a book, article, or web page.
//! It is the basic unit of bibliographic data.
//!
//! The model includes the following core data types.
//! Each is designed to be as simple as possible, while also allowing more complex data structures.
//!
//! ## Title
//!
//! A title can be a single string, a structured title, or a multilingual title.
//!
//! ## Contributor
//!
//! A contributor can be a single string, a structured name, or a list of contributors.
//!
//! ## Date
//!
//! Dates can either be EDTF strings, for flexible dates and date-times, or literal strings.
//! Literal strings can be used for examples like "Han Dynasty".
//!
//! ## Parent References
//!
//! A reference can be a component of a larger work, such as a chapter in a book, or an article.
//! The parent is represented inline as a Monograph or Serial.
//! TODO: Add ability to reference a parent by ID.

use crate::locale::MonthList;
use crate::options::{AndOptions, AndOtherOptions, Config, DisplayAsSort};
use edtf::level_1::Edtf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use url::Url;

/// The Reference model.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum InputReference {
    /// A monograph, such as a book or a report, is a monolithic work published or produced as a complete entity.
    Monograph(Box<Monograph>),
    /// A component of a larger Monograph, such as a chapter in a book.
    /// The parent monograph is referenced by its ID.
    CollectionComponent(Box<CollectionComponent>),
    /// A component of a larger serial publication; for example a journal or newspaper article.
    /// The parent serial is referenced by its ID.
    SerialComponent(Box<SerialComponent>),
    /// A collection of works, such as an anthology or proceedings.
    Collection(Box<Collection>),
}

impl InputReference {
    /// Return the reference ID.
    /// If the reference does not have an ID, return None.
    pub fn id(&self) -> Option<RefID> {
        match self {
            InputReference::Monograph(r) => r.id.clone(),
            InputReference::CollectionComponent(r) => r.id.clone(),
            InputReference::SerialComponent(r) => r.id.clone(),
            InputReference::Collection(r) => r.id.clone(),
        }
    }

    /// Return the author.
    /// If the reference does not have an author, return None.
    pub fn author(&self) -> Option<Contributor> {
        match self {
            InputReference::Monograph(r) => r.author.clone(),
            InputReference::CollectionComponent(r) => r.author.clone(),
            InputReference::SerialComponent(r) => r.author.clone(),
            _ => None,
        }
    }

    /// Return the editor.
    /// If the reference does not have an editor, return None.
    pub fn editor(&self) -> Option<Contributor> {
        match self {
            InputReference::Collection(r) => r.editor.clone(),
            InputReference::CollectionComponent(r) => r.parent.editor.clone(),
            _ => None,
        }
    }

    /// Return the translator.
    /// If the reference does not have a translator, return None.
    pub fn translator(&self) -> Option<Contributor> {
        match self {
            InputReference::Monograph(r) => r.translator.clone(),
            InputReference::CollectionComponent(r) => r.translator.clone(),
            InputReference::SerialComponent(r) => r.translator.clone(),
            InputReference::Collection(r) => r.translator.clone(),
        }
    }

    /// Return the publisher.
    /// If the reference does not have a publisher, return None.
    pub fn publisher(&self) -> Option<Contributor> {
        match self {
            InputReference::Monograph(r) => r.publisher.clone(),
            InputReference::CollectionComponent(r) => r.parent.publisher.clone(),
            InputReference::Collection(r) => r.publisher.clone(),
            _ => None,
        }
    }

    /// Return the title.
    /// If the reference does not have a title, return None.
    pub fn title(&self) -> Option<Title> {
        match self {
            InputReference::Monograph(r) => Some(r.title.clone()),
            InputReference::CollectionComponent(r) => r.title.clone(),
            InputReference::SerialComponent(r) => r.title.clone(),
            InputReference::Collection(r) => r.title.clone(),
        }
    }

    /// Return the issued date.
    /// If the reference does not have an issued date, return None.
    pub fn issued(&self) -> Option<EdtfString> {
        match self {
            InputReference::Monograph(r) => Some(r.issued.clone()),
            InputReference::CollectionComponent(r) => Some(r.issued.clone()),
            InputReference::SerialComponent(r) => Some(r.issued.clone()),
            InputReference::Collection(r) => Some(r.issued.clone()),
        }
    }

    /// Set the reference ID.
    pub fn set_id(&mut self, id: String) {
        match self {
            InputReference::Monograph(monograph) => monograph.id = Some(id),
            InputReference::CollectionComponent(component) => component.id = Some(id),
            InputReference::SerialComponent(component) => component.id = Some(id),
            InputReference::Collection(collection) => collection.id = Some(id),
        }
    }
}

/// A value that could be either a number or a string.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NumOrStr {
    /// It's a number!
    Number(i64),
    /// It's a string!
    Str(String),
}

impl Display for NumOrStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Number(i) => write!(f, "{}", i),
            Self::Str(s) => write!(f, "{}", s),
        }
    }
}

/// A monograph, such as a book or a report, is a monolithic work published or produced as a complete entity.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Monograph {
    pub id: Option<RefID>,
    pub r#type: MonographType,
    pub title: Title,
    pub author: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    pub publisher: Option<Contributor>,
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub note: Option<String>,
    pub isbn: Option<String>,
    pub doi: Option<String>,
    pub edition: Option<String>,
}

/// A collection of works, such as an anthology or proceedings.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Collection {
    pub id: Option<RefID>,
    pub r#type: CollectionType,
    pub title: Option<Title>,
    pub editor: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    pub publisher: Option<Contributor>,
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub note: Option<String>,
    pub isbn: Option<String>,
}

/// Types of collections.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CollectionType {
    Anthology,
    Proceedings,
    EditedBook,
    EditedVolume,
}

/// A component of a larger serial publication; for example a journal or newspaper article.
/// The parent serial is referenced by its ID.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct SerialComponent {
    pub id: Option<RefID>,
    pub r#type: SerialComponentType,
    pub title: Option<Title>,
    pub author: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    /// The parent work, such as a magazine or journal.
    pub parent: Serial,
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub note: Option<String>,
    pub doi: Option<String>,
    pub pages: Option<String>,
    pub volume: Option<NumOrStr>,
    pub issue: Option<NumOrStr>,
}

/// A parent reference (either Monograph or Serial).
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum ParentReference {
    Monograph(Box<Monograph>),
    Serial(Serial),
}

/// Types of serial components.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SerialComponentType {
    Article,
    Post,
    Review,
}

/// A serial publication (journal, magazine, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Serial {
    pub r#type: SerialType,
    pub title: Title,
}

/// Types of serial publications.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SerialType {
    AcademicJournal,
    Blog,
    Magazine,
    Newspaper,
    Newsletter,
    Proceedings,
    Podcast,
    BroadcastProgram,
}

/// Types of monograph components.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MonographComponentType {
    Chapter,
    /// A generic part of a monograph, such as a preface or an appendix.
    Document,
    Section,
    Part,
}

/// Types of monographs.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MonographType {
    #[default]
    Book,
    /// A standalone generic item.
    Document,
    Report,
}

/// A component of a larger Monograph, such as a chapter in a book.
/// The parent monograph is referenced by its ID.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct CollectionComponent {
    pub id: Option<RefID>,
    pub r#type: MonographComponentType,
    pub title: Option<Title>,
    pub author: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    /// The parent work, as either a Monograph.
    pub parent: Collection,
    pub pages: Option<NumOrStr>,
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub note: Option<String>,
    pub doi: Option<String>,
}

/// A reference ID (citekey).
pub type RefID = String;

/// A locale/language identifier.
pub type LangID = String;

/// A collection of formattable strings consisting of a title, a translated title, and a shorthand.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Title {
    /// A title in a single language.
    Single(String),
    /// A structured title.
    Structured(StructuredTitle),
    /// A title in multiple languages.
    Multi(Vec<(LangID, String)>),
    /// A structured title in multiple languages.
    MultiStructured(Vec<(LangID, StructuredTitle)>),
    /// An abbreviated title (shorthand, full).
    Shorthand(String, String),
}

/// Where title parts are meaningful, use this struct; CSLN processors will not parse title strings.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct StructuredTitle {
    pub full: Option<String>,
    pub main: String,
    pub sub: Subtitle,
}

/// The subtitle can either be a string, as is the common case, or a vector of strings.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum Subtitle {
    String(String),
    Vector(Vec<String>),
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Title::Single(s) => write!(f, "{}", s),
            Title::Multi(_m) => write!(f, "[multilingual title]"),
            Title::Structured(s) => {
                let subtitle = match &s.sub {
                    Subtitle::String(s) => s.clone(),
                    Subtitle::Vector(v) => v.join(", "),
                };
                write!(f, "{}: {}", s.main, subtitle)
            }
            Title::MultiStructured(_m) => write!(f, "[multilingual structured title]"),
            Title::Shorthand(s, t) => write!(f, "{} ({})", s, t),
        }
    }
}

/// A string conforming to the EDTF specification.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
pub struct EdtfString(pub String);

/// Date inputs must be valid EDTF strings, or a literal string.
#[derive(Debug, PartialEq)]
pub enum RefDate {
    Edtf(Edtf),
    Literal(String),
}

impl EdtfString {
    /// Parse the string as an EDTF date etc, or return the string as a literal.
    pub fn parse(&self) -> RefDate {
        match Edtf::parse(&self.0) {
            Ok(edtf) => RefDate::Edtf(edtf),
            Err(_) => RefDate::Literal(self.0.clone()),
        }
    }

    fn component_to_u32(&self, component: Option<edtf::level_1::Component>) -> u32 {
        match component {
            Some(component) => component.value().unwrap_or(0),
            None => 0,
        }
    }

    /// Extract the year from the date.
    pub fn year(&self) -> String {
        let parsed_date = self.parse();
        match parsed_date {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => date.year().to_string(),
                Edtf::YYear(year) => format!("{}", year.value()),
                Edtf::DateTime(datetime) => datetime.date().year().to_string(),
                Edtf::Interval(start, _end) => format!("{}", start.year()),
                Edtf::IntervalFrom(date, _terminal) => format!("{}", date.year()),
                Edtf::IntervalTo(_terminal, date) => format!("{}", date.year()),
            },
            RefDate::Literal(_) => String::new(),
        }
    }

    fn month_to_string(month: u32, months: MonthList) -> String {
        if month > 0 {
            let index = month - 1;
            if index < months.len() as u32 {
                months[index as usize].clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    /// Extract the month from the date.
    pub fn month(&self, months: MonthList) -> String {
        let parsed_date = self.parse();
        let month: Option<u32> = match parsed_date {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => Some(self.component_to_u32(date.month())),
                Edtf::YYear(_year) => None,
                Edtf::DateTime(datetime) => Some(datetime.date().month()),
                Edtf::Interval(_start, _end) => None,
                Edtf::IntervalFrom(_date, _terminal) => None,
                Edtf::IntervalTo(_terminal, _date) => None,
            },
            RefDate::Literal(_) => None,
        };
        match month {
            Some(month) => EdtfString::month_to_string(month, months),
            None => String::new(),
        }
    }

    /// Format as "Month Year".
    pub fn year_month(&self, months: MonthList) -> String {
        let month = self.month(months);
        let year = self.year();
        if month.is_empty() || year.is_empty() {
            String::new()
        } else {
            format!("{} {}", month, year)
        }
    }

    /// Extract the day from the date.
    pub fn day(&self) -> Option<u32> {
        let parsed_date = self.parse();
        match parsed_date {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => Some(self.component_to_u32(date.day())),
                Edtf::YYear(_) => None,
                Edtf::DateTime(datetime) => Some(datetime.date().day()),
                Edtf::Interval(_, _) => None,
                Edtf::IntervalFrom(_, _) => None,
                Edtf::IntervalTo(_, _) => None,
            },
            RefDate::Literal(_) => None,
        }
        .filter(|&d| d > 0)
    }

    /// Format as "Month Day".
    pub fn month_day(&self, months: MonthList) -> String {
        let month = self.month(months);
        let day = self.day();
        match day {
            Some(d) if !month.is_empty() => format!("{} {}", month, d),
            _ => String::new(),
        }
    }
}

impl RefDate {
    /// Apply a function to the EDTF date if present.
    pub fn and_then<F, T>(self, f: F) -> Option<T>
    where
        F: FnOnce(Edtf) -> Option<T>,
    {
        match self {
            RefDate::Edtf(edtf) => f(edtf),
            RefDate::Literal(_) => None,
        }
    }

    /// Extract the year as an integer.
    pub fn year(&self) -> i32 {
        match self {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => date.year(),
                Edtf::YYear(year) => year.value() as i32,
                Edtf::DateTime(datetime) => datetime.date().year(),
                Edtf::Interval(start, _end) => start.year(),
                Edtf::IntervalFrom(date, _terminal) => date.year(),
                Edtf::IntervalTo(_terminal, date) => date.year(),
            },
            // Since we need this for sorting, return 0 for now.
            RefDate::Literal(_) => 0,
        }
    }
}

impl fmt::Display for EdtfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parsed_date: Edtf = match Edtf::parse(&self.0) {
            Ok(edtf) => edtf,
            Err(_) => return write!(f, "{}", self.0),
        };
        write!(f, "{}", parsed_date)
    }
}

/// A contributor can be a person or an organization.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum Contributor {
    SimpleName(SimpleName),
    StructuredName(StructuredName),
    ContributorList(ContributorList),
}

/// A simple (literal) name, typically for organizations.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct SimpleName {
    pub name: String,
    pub location: Option<String>,
}

/// The contributor list model.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
pub struct ContributorList(pub Vec<Contributor>);

/// Structured personal contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct StructuredName {
    pub given: String,
    pub family: String,
    /// Name suffix (Jr., III, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Dropping particle (de, van, etc. that sorts with given name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dropping_particle: Option<String>,
    /// Non-dropping particle (de, van, etc. that sorts with family name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_dropping_particle: Option<String>,
}

impl fmt::Display for Contributor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Contributor::SimpleName(c) => write!(f, "{}", c.name),
            Contributor::StructuredName(c) => write!(f, "{} {}", c.given, c.family),
            Contributor::ContributorList(c) => write!(f, "{}", c),
        }
    }
}

impl StructuredName {
    /// Return the initials of the name.
    pub fn initials(&self, with: Option<String>) -> String {
        let with = with.unwrap_or_default();
        let initials = self
            .given
            .split_whitespace()
            .map(|name| name.chars().next().unwrap_or_default())
            .collect::<Vec<char>>();
        let initials_string = initials
            .iter()
            .map(|&c| c.to_string())
            .collect::<Vec<String>>()
            .join(&with)
            + &with;
        initials_string
    }
}

impl fmt::Display for ContributorList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contributors: Vec<String> = self.0.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", contributors.join(", "))
    }
}

impl Contributor {
    /// Get list of formatted names.
    pub fn names(&self, options: &Config, as_sorted: bool) -> Vec<String> {
        match self {
            Contributor::SimpleName(c) => vec![c.name.to_string()],
            Contributor::StructuredName(c) => {
                if as_sorted {
                    vec![format!("{}, {}", c.family, c.given)]
                } else {
                    vec![format!("{} {}", c.given, c.family)]
                }
            }
            Contributor::ContributorList(contributors) => contributors.names_list(options),
        }
    }

    /// Join a vector of strings with commas and "and".
    pub fn name_list_and(&self, and: String) -> Vec<String> {
        let names = self.names(&Config::default(), false);
        let mut result = names;
        if result.len() > 1 {
            if let Some(last) = result.pop() {
                result.push(format!("{} {}", and, last));
            }
        }
        result
    }

    /// Shorten a name list to the first N names.
    pub fn name_list_shorten(&self, names: &[&str], use_first: u8) -> Vec<String> {
        names
            .iter()
            .take(use_first as usize)
            .map(|&s| s.to_string())
            .collect()
    }

    fn format_list(&self, names: Vec<String>, and_str: String, oxford_comma: bool) -> String {
        let last = names.last().map(ToString::to_string).unwrap_or_default();
        match names.len() {
            0 => String::new(),
            1 => last,
            2 => format!("{} {} {}", names[0], and_str, last),
            _ => {
                let all_but_last = names[..names.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                if oxford_comma {
                    format!("{}, {} {}", all_but_last, and_str, last)
                } else {
                    format!("{} {} {}", all_but_last, and_str, last)
                }
            }
        }
    }

    /// Format the contributor for display.
    pub fn format(&self, options: &Config) -> String {
        let as_sorted: bool = matches!(self, Contributor::StructuredName(_));
        let names = self.names(options, as_sorted);
        let contributor_options = options.contributors.clone().unwrap_or_default();
        let shorten: bool =
            contributor_options.shorten.unwrap_or_default().min <= names.len() as u8;
        if shorten {
            let shorten_options = options
                .contributors
                .clone()
                .unwrap_or_default()
                .shorten
                .unwrap_or_default();
            let use_first = shorten_options.use_first;
            let and_others = shorten_options.and_others;
            let and_others_string = match and_others {
                AndOtherOptions::EtAl => "et al.".to_string(),
                AndOtherOptions::Text => "and others".to_string(),
            };
            let names_str: Vec<&str> = names.iter().map(AsRef::as_ref).collect();
            let result = self.name_list_shorten(&names_str, use_first);
            format!("{} {}", result.join(", "), and_others_string)
        } else {
            let and_options = contributor_options.and;
            let and_string = match and_options {
                Some(AndOptions::Symbol) => "&".to_string(),
                Some(AndOptions::Text) => "and".to_string(),
                _ => String::new(),
            };
            self.format_list(names, and_string, true)
        }
    }
}

impl ContributorList {
    fn as_sorted(options: &Config, index: usize) -> bool {
        let display_as_sort = options
            .contributors
            .clone()
            .unwrap_or_default()
            .display_as_sort;
        index == 0 && display_as_sort == Some(DisplayAsSort::First)
            || display_as_sort == Some(DisplayAsSort::All)
    }

    /// Get names formatted according to options.
    pub fn names_list(&self, options: &Config) -> Vec<String> {
        self.0
            .iter()
            .enumerate()
            .flat_map(|(i, c)| c.names(options, Self::as_sorted(options, i)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_year_from_edtf_dates() {
        let date = EdtfString("2020-01-01".to_string()).parse();
        assert_eq!(date.year(), 2020);
        let date = EdtfString("2021-10".to_string()).parse();
        assert_eq!(date.year(), 2021);
        let date = EdtfString("2022".to_string()).parse();
        assert_eq!(date.year(), 2022);
    }

    #[test]
    fn test_literal_dates() {
        let date_string = EdtfString("foo bar".to_string());
        assert_eq!(date_string.parse(), RefDate::Literal("foo bar".to_string()));
    }

    #[test]
    fn test_initials() {
        let name = StructuredName {
            given: "Jane Mary".to_string(),
            family: "Smith".to_string(),
            ..Default::default()
        };
        assert_eq!(name.initials(None), "JM");
        assert_eq!(name.initials(Some(".".to_string())), "J.M.");
    }

    #[test]
    fn test_contributor_name() {
        let contributor = Contributor::SimpleName(SimpleName {
            name: "ABC".to_string(),
            location: None,
        });
        assert_eq!(contributor.to_string(), "ABC");

        let contributor = Contributor::StructuredName(StructuredName {
            given: "John".to_string(),
            family: "Smith".to_string(),
            ..Default::default()
        });
        assert_eq!(contributor.to_string(), "John Smith");

        let contributor = Contributor::ContributorList(ContributorList(vec![
            Contributor::SimpleName(SimpleName {
                name: "John Smith".to_string(),
                location: None,
            }),
            Contributor::SimpleName(SimpleName {
                name: "Jane Smith".to_string(),
                location: None,
            }),
        ]));
        assert_eq!(contributor.to_string(), "John Smith, Jane Smith");
    }

    #[test]
    fn test_year_months() {
        let months: MonthList = vec![
            "January".to_string(),
            "February".to_string(),
            "March".to_string(),
            "April".to_string(),
            "May".to_string(),
            "June".to_string(),
            "July".to_string(),
            "August".to_string(),
            "September".to_string(),
            "October".to_string(),
            "November".to_string(),
            "December".to_string(),
        ];
        let date = EdtfString("2020-01-01".to_string());
        assert_eq!(date.year_month(months), "January 2020");
    }

    #[test]
    fn test_month_from_edtf_dates() {
        let months: MonthList = vec![
            "January".to_string(),
            "February".to_string(),
            "March".to_string(),
            "April".to_string(),
            "May".to_string(),
            "June".to_string(),
            "July".to_string(),
            "August".to_string(),
            "September".to_string(),
            "October".to_string(),
            "November".to_string(),
            "December".to_string(),
        ];
        let date = EdtfString("2020-01-01".to_string());
        assert_eq!(date.month(months), "January");
    }

    #[test]
    fn test_day_from_edtf_dates() {
        // Full date: day should be extracted
        let date = EdtfString("2020-03-15".to_string());
        assert_eq!(date.day(), Some(15));

        // Year-month only: no day
        let date_ym = EdtfString("2020-03".to_string());
        assert_eq!(date_ym.day(), None);

        // Year only: no day
        let date_y = EdtfString("2020".to_string());
        assert_eq!(date_y.day(), None);

        // Literal date: no day
        let literal = EdtfString("Han Dynasty".to_string());
        assert_eq!(literal.day(), None);
    }

    #[test]
    fn test_month_day_format() {
        let months: MonthList = vec![
            "January".to_string(),
            "February".to_string(),
            "March".to_string(),
            "April".to_string(),
            "May".to_string(),
            "June".to_string(),
            "July".to_string(),
            "August".to_string(),
            "September".to_string(),
            "October".to_string(),
            "November".to_string(),
            "December".to_string(),
        ];

        // Full date: should format as "Month Day"
        let date = EdtfString("2020-03-15".to_string());
        assert_eq!(date.month_day(months.clone()), "March 15");

        // Year-month only: returns empty (no day)
        let date_ym = EdtfString("2020-03".to_string());
        assert_eq!(date_ym.month_day(months.clone()), "");

        // Year only: returns empty
        let date_y = EdtfString("2020".to_string());
        assert_eq!(date_y.month_day(months), "");
    }

    #[test]
    fn test_display_and_sort_names() {
        let simple = Contributor::SimpleName(SimpleName {
            name: "John Doe".to_string(),
            location: None,
        });
        let structured = Contributor::StructuredName(StructuredName {
            given: "John".to_string(),
            family: "Doe".to_string(),
            ..Default::default()
        });
        let options = Config::default();
        assert_eq!(simple.names(&options, false).join(" "), "John Doe");
        assert_eq!(
            simple.names(&options, true).join(" "),
            "John Doe",
            "as_sorted=true should not affect a simple name"
        );
        assert_eq!(structured.names(&options, false).join(" "), "John Doe");
        assert_eq!(structured.names(&options, true).join(", "), "Doe, John");
    }
}
