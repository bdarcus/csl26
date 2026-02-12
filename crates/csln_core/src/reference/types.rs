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
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
    pub isbn: Option<String>,
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
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
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
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
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
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub language: Option<LangID>,
    pub note: Option<String>,
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
    Serial(Serial),
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
    Edtf(edtf::level_1::Edtf),
    Literal(String),
}
