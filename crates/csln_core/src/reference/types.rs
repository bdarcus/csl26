use crate::reference::contributor::Contributor;
use crate::reference::date::EdtfString;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use url::Url;

pub type RefID = String;
pub type LangID = String;

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

/// A string that can be represented in multiple languages and scripts.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum MultilingualString {
    Simple(String),
    Complex(MultilingualComplex),
}

/// Complex multilingual representation with original, transliterations, and translations.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualComplex {
    /// The text in its original script.
    pub original: String,
    /// ISO 639/BCP 47 language code for the original text.
    pub lang: Option<LangID>,
    /// Transliterations/Transcriptions of the original text.
    /// Keys are script codes or full BCP 47 tags.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub transliterations: HashMap<String, String>,
    /// Translations of the text into other languages.
    /// Keys are ISO 639/BCP 47 language codes.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub translations: HashMap<LangID, String>,
}

impl From<String> for MultilingualString {
    fn from(s: String) -> Self {
        Self::Simple(s)
    }
}

impl From<&str> for MultilingualString {
    fn from(s: &str) -> Self {
        Self::Simple(s.to_string())
    }
}

impl Display for MultilingualString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Simple(s) => write!(f, "{}", s),
            Self::Complex(c) => write!(f, "{}", c.original),
        }
    }
}

impl Default for MultilingualString {
    fn default() -> Self {
        Self::Simple(String::new())
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
    pub editor: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    pub publisher: Option<Contributor>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    #[serde(alias = "ISBN")]
    pub isbn: Option<String>,
    #[serde(alias = "DOI")]
    pub doi: Option<String>,
    pub edition: Option<String>,
    pub genre: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub original_date: Option<EdtfString>,
    pub original_title: Option<Title>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MonographType {
    Book,
    Report,
    Thesis,
    Webpage,
    Post,
    Document,
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
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    #[serde(alias = "ISBN")]
    pub isbn: Option<String>,
    pub keywords: Option<Vec<String>>,
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

/// A component of a larger monograph, such as a chapter in a book.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct CollectionComponent {
    pub id: Option<RefID>,
    pub r#type: MonographComponentType,
    pub title: Option<Title>,
    pub author: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    pub parent: Parent<Collection>,
    pub pages: Option<NumOrStr>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    #[serde(alias = "DOI")]
    pub doi: Option<String>,
    pub genre: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// Types of monograph components.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MonographComponentType {
    Chapter,
    Document,
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
    pub parent: Parent<Serial>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    #[serde(alias = "DOI")]
    pub doi: Option<String>,
    pub pages: Option<String>,
    pub volume: Option<NumOrStr>,
    pub issue: Option<NumOrStr>,
    pub keywords: Option<Vec<String>>,
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
    pub editor: Option<Contributor>,
    pub publisher: Option<Contributor>,
    #[serde(alias = "ISSN")]
    pub issn: Option<String>,
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

/// A parent reference (either embedded or by ID).
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum Parent<T> {
    Embedded(T),
    Id(RefID),
}

/// A parent reference (either Monograph or Serial).
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum ParentReference {
    Monograph(Box<Monograph>),
    Serial(Box<Serial>),
}

/// A title can be a single string, a structured title, or a multilingual title.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum Title {
    /// A title in a single language.
    Single(String),
    /// A structured title.
    Structured(StructuredTitle),
    /// A complex multilingual title.
    Multilingual(MultilingualComplex),
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
            Title::Multilingual(m) => write!(f, "{}", m.original),
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

/// Date type.
#[derive(Debug, Clone, PartialEq)]
pub enum RefDate {
    Edtf(csln_edtf::Edtf),
    Literal(String),
}

/// A legal case (court decision).
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct LegalCase {
    pub id: Option<RefID>,
    /// Case name (e.g., "Brown v. Board of Education")
    pub title: Title,
    /// Court or authority (e.g., "U.S. Supreme Court")
    pub authority: String,
    /// Reporter volume
    pub volume: Option<String>,
    /// Reporter abbreviation (e.g., "U.S.", "F.2d")
    pub reporter: Option<String>,
    /// First page of case in reporter
    pub page: Option<String>,
    /// Decision date
    pub issued: EdtfString,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    #[serde(alias = "DOI")]
    pub doi: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A statute or legislative act.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Statute {
    pub id: Option<RefID>,
    /// Statute name (e.g., "Civil Rights Act of 1964")
    pub title: Title,
    /// Legislative body (e.g., "U.S. Congress")
    pub authority: Option<String>,
    /// Code volume
    pub volume: Option<String>,
    /// Code abbreviation (e.g., "U.S.C.", "Pub. L.")
    pub code: Option<String>,
    /// Section or page number
    pub section: Option<String>,
    /// Enactment or publication date
    pub issued: EdtfString,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// An international treaty or agreement.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Treaty {
    pub id: Option<RefID>,
    /// Treaty name (e.g., "Treaty of Versailles")
    pub title: Title,
    /// Parties to the treaty
    pub author: Option<Contributor>,
    /// Treaty series volume
    pub volume: Option<String>,
    /// Treaty series abbreviation (e.g., "U.N.T.S.")
    pub reporter: Option<String>,
    /// Page or treaty number
    pub page: Option<String>,
    /// Signing or ratification date
    pub issued: EdtfString,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A legislative or administrative hearing.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Hearing {
    pub id: Option<RefID>,
    /// Hearing title
    pub title: Title,
    /// Legislative body conducting the hearing (e.g., "U.S. Senate Committee on Finance")
    pub authority: Option<String>,
    /// Session or congress number
    pub number: Option<String>,
    /// Hearing date
    pub issued: EdtfString,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// An administrative regulation.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Regulation {
    pub id: Option<RefID>,
    /// Regulation title
    pub title: Title,
    /// Regulatory authority (e.g., "EPA", "Federal Register")
    pub authority: Option<String>,
    /// Code volume
    pub volume: Option<String>,
    /// Code abbreviation (e.g., "C.F.R.", "Fed. Reg.")
    pub code: Option<String>,
    /// Section or page number
    pub section: Option<String>,
    /// Publication or effective date
    pub issued: EdtfString,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A legal brief or filing.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Brief {
    pub id: Option<RefID>,
    /// Brief title or case name
    pub title: Title,
    /// Court (e.g., "U.S. Supreme Court")
    pub authority: Option<String>,
    /// Author/filer of the brief
    pub author: Option<Contributor>,
    /// Docket number
    pub number: Option<String>,
    /// Filing date
    pub issued: EdtfString,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A classic work (Aristotle, Bible, etc.) with standard citation forms.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Classic {
    pub id: Option<RefID>,
    /// Work title (e.g., "Nicomachean Ethics")
    pub title: Title,
    /// Author (e.g., "Aristotle")
    pub author: Option<Contributor>,
    /// Editor or translator
    pub editor: Option<Contributor>,
    pub translator: Option<Contributor>,
    /// Volume in standard reference system
    pub volume: Option<String>,
    /// Section, book, or chapter in standard reference system
    pub section: Option<String>,
    /// Publication date of this edition (not original)
    pub issued: EdtfString,
    pub publisher: Option<Contributor>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A patent.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Patent {
    pub id: Option<RefID>,
    /// Patent title
    pub title: Title,
    /// Inventor(s)
    pub author: Option<Contributor>,
    /// Assignee (patent holder)
    pub assignee: Option<Contributor>,
    /// Patent number (e.g., "U.S. Patent No. 7,347,809")
    pub number: String,
    /// Application number
    pub application_number: Option<String>,
    /// Filing date
    pub filing_date: Option<EdtfString>,
    /// Issue/grant date
    pub issued: EdtfString,
    /// Jurisdiction (e.g., "US", "EP", "JP")
    pub jurisdiction: Option<String>,
    /// Patent office (e.g., "U.S. Patent and Trademark Office")
    pub authority: Option<String>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A research dataset.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Dataset {
    pub id: Option<RefID>,
    /// Dataset title
    pub title: Title,
    /// Dataset author(s)/creator(s)
    pub author: Option<Contributor>,
    /// Publication/release date
    pub issued: EdtfString,
    /// Publisher or repository (e.g., "Zenodo", "Dryad")
    pub publisher: Option<Contributor>,
    /// Version number
    pub version: Option<String>,
    /// File format (e.g., "CSV", "NetCDF", "HDF5")
    pub format: Option<String>,
    /// Dataset size (e.g., "2.4 GB", "150,000 records")
    pub size: Option<String>,
    /// Repository or archive name
    pub repository: Option<String>,
    #[serde(alias = "DOI")]
    pub doi: Option<String>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// A technical standard or specification.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Standard {
    pub id: Option<RefID>,
    /// Standard title
    pub title: Title,
    /// Standards organization (e.g., "ISO", "ANSI", "IEEE")
    pub authority: Option<String>,
    /// Standard number (e.g., "ISO 8601", "IEEE 754-2008")
    pub number: String,
    /// Publication date
    pub issued: EdtfString,
    /// Status (e.g., "published", "draft", "withdrawn")
    pub status: Option<String>,
    /// Publisher (usually same as authority)
    pub publisher: Option<Contributor>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}

/// Software or source code.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Software {
    pub id: Option<RefID>,
    /// Software title
    pub title: Title,
    /// Author(s)/developer(s)
    pub author: Option<Contributor>,
    /// Release date
    pub issued: EdtfString,
    /// Publisher or repository (e.g., "GitHub", "Zenodo")
    pub publisher: Option<Contributor>,
    /// Version number (e.g., "4.1.0", "v2.3.1")
    pub version: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// License (e.g., "MIT", "GPL-3.0", "Apache-2.0")
    pub license: Option<String>,
    /// Platform (e.g., "Windows", "macOS", "Linux", "cross-platform")
    pub platform: Option<String>,
    #[serde(alias = "DOI")]
    pub doi: Option<String>,
    #[serde(alias = "URL")]
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub keywords: Option<Vec<String>>,
}
