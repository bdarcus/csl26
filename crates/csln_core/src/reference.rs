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
//!
//! Future enhancement: support referencing a parent by ID to reduce duplication.
//! See: https://github.com/bdarcus/csl26/issues/64

use crate::locale::MonthList;
use crate::options::{AndOptions, AndOtherOptions, Config, DisplayAsSort};
use biblatex::{Chunk, Entry, Person};
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

    pub fn editor(&self) -> Option<Contributor> {
        match self {
            InputReference::Monograph(r) => r.editor.clone(),
            InputReference::Collection(r) => r.editor.clone(),
            InputReference::CollectionComponent(r) => match &(**r).parent {
                Parent::Embedded(p) => p.editor.clone(),
                Parent::Id(_) => None,
            },
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
            InputReference::CollectionComponent(r) => {
                let r = r.as_ref();
                match &r.parent {
                    Parent::Embedded(p) => p.publisher.clone(),
                    Parent::Id(_) => None,
                }
            }
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

    /// Return the DOI.
    pub fn doi(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.doi.clone(),
            InputReference::CollectionComponent(r) => r.doi.clone(),
            InputReference::SerialComponent(r) => r.doi.clone(),
            InputReference::Collection(_) => None,
        }
    }

    /// Return the URL.
    pub fn url(&self) -> Option<Url> {
        match self {
            InputReference::Monograph(r) => r.url.clone(),
            InputReference::CollectionComponent(r) => r.url.clone(),
            InputReference::SerialComponent(r) => r.url.clone(),
            InputReference::Collection(r) => r.url.clone(),
        }
    }

    /// Return the container title.
    /// Return the publisher place.
    pub fn publisher_place(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.publisher.as_ref().and_then(|c| c.location()),
            InputReference::CollectionComponent(r) => {
                // Use publisher from parent
                match &r.parent {
                    Parent::Embedded(p) => p.publisher.as_ref().and_then(|c| c.location()),
                    _ => None,
                }
            }
            InputReference::SerialComponent(_) => None,
            InputReference::Collection(r) => r.publisher.as_ref().and_then(|c| c.location()),
        }
    }

    /// Return the publisher as a string.
    pub fn publisher_str(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.publisher.as_ref().and_then(|c| c.name()),
            InputReference::CollectionComponent(r) => {
                // Use publisher from parent
                match &r.parent {
                    Parent::Embedded(p) => p.publisher.as_ref().and_then(|c| c.name()),
                    _ => None,
                }
            }
            InputReference::SerialComponent(_) => None,
            InputReference::Collection(r) => r.publisher.as_ref().and_then(|c| c.name()),
        }
    }

    /// Return the genre/type as string.
    pub fn genre(&self) -> Option<String> {
        Some(self.ref_type())
    }

    /// Return the abstract.
    pub fn abstract_text(&self) -> Option<String> {
        None // Not currently supported in structs
    }

    pub fn container_title(&self) -> Option<Title> {
        match self {
            InputReference::CollectionComponent(r) => {
                let r = r.as_ref();
                match &r.parent {
                    Parent::Embedded(p) => p.title.clone(),
                    Parent::Id(_) => None,
                }
            }
            InputReference::SerialComponent(r) => {
                let r = r.as_ref();
                match &r.parent {
                    Parent::Embedded(p) => Some(p.title.clone()),
                    Parent::Id(_) => None,
                }
            }
            _ => None,
        }
    }

    /// Return the volume.
    pub fn volume(&self) -> Option<NumOrStr> {
        match self {
            InputReference::SerialComponent(r) => r.volume.clone(),
            _ => None,
        }
    }

    /// Return the issue.
    pub fn issue(&self) -> Option<NumOrStr> {
        match self {
            InputReference::SerialComponent(r) => r.issue.clone(),
            _ => None,
        }
    }

    /// Return the pages.
    pub fn pages(&self) -> Option<NumOrStr> {
        match self {
            InputReference::CollectionComponent(r) => r.pages.clone(),
            InputReference::SerialComponent(r) => r.pages.clone().map(NumOrStr::Str),
            _ => None,
        }
    }

    /// Return the edition.
    pub fn edition(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.edition.clone(),
            _ => None,
        }
    }

    /// Return the accessed date.
    pub fn accessed(&self) -> Option<EdtfString> {
        match self {
            InputReference::Monograph(r) => r.accessed.clone(),
            InputReference::CollectionComponent(r) => r.accessed.clone(),
            InputReference::SerialComponent(r) => r.accessed.clone(),
            InputReference::Collection(r) => r.accessed.clone(),
        }
    }

    /// Return the original publication date.
    pub fn original_date(&self) -> Option<EdtfString> {
        match self {
            InputReference::Monograph(r) => r.original_date.clone(),
            _ => None,
        }
    }

    /// Return the ISBN.
    pub fn isbn(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.isbn.clone(),
            _ => None,
        }
    }

    /// Return the ISSN.
    pub fn issn(&self) -> Option<String> {
        match self {
            InputReference::SerialComponent(r) => match &r.parent {
                Parent::Embedded(s) => s.issn.clone(),
                Parent::Id(_) => None,
            },
            _ => None,
        }
    }

    /// Return the Keywords.
    pub fn keywords(&self) -> Option<Vec<String>> {
        match self {
            InputReference::Monograph(r) => r.keywords.clone(),
            InputReference::CollectionComponent(r) => r.keywords.clone(),
            InputReference::SerialComponent(r) => r.keywords.clone(),
            InputReference::Collection(r) => r.keywords.clone(),
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

    /// Return the reference type as a string (CSL-compatible).
    pub fn ref_type(&self) -> String {
        match self {
            InputReference::Monograph(r) => match (**r).r#type {
                MonographType::Book => "book".to_string(),
                MonographType::Report => "report".to_string(),
                MonographType::Document => "document".to_string(),
            },
            InputReference::CollectionComponent(r) => match (**r).r#type {
                MonographComponentType::Chapter => "chapter".to_string(),
                _ => "chapter".to_string(),
            },
            InputReference::SerialComponent(r) => match (**r).r#type {
                SerialComponentType::Article => "article-journal".to_string(),
                _ => "article".to_string(),
            },
            InputReference::Collection(r) => match (**r).r#type {
                CollectionType::EditedBook => "book".to_string(),
                _ => "collection".to_string(),
            },
        }
    }
}

impl From<csl_legacy::csl_json::Reference> for InputReference {
    fn from(legacy: csl_legacy::csl_json::Reference) -> Self {
        let id = Some(legacy.id);
        let title = legacy
            .title
            .map(Title::Single)
            .unwrap_or(Title::Single(String::new()));
        let issued = legacy
            .issued
            .map(EdtfString::from)
            .unwrap_or(EdtfString(String::new()));
        let url = legacy.url.and_then(|u| Url::parse(&u).ok());
        let accessed = legacy.accessed.map(EdtfString::from);
        let note = legacy.note;
        let doi = legacy.doi;
        let isbn = legacy.isbn;
        let edition = legacy.edition.map(|e| e.to_string());

        match legacy.ref_type.as_str() {
            "book" | "report" => {
                let r#type = if legacy.ref_type == "report" {
                    MonographType::Report
                } else {
                    MonographType::Book
                };
                InputReference::Monograph(Box::new(Monograph {
                    id,
                    r#type,
                    title,
                    author: legacy.author.map(Contributor::from),
                    editor: legacy.editor.map(Contributor::from),
                    translator: legacy.translator.map(Contributor::from),
                    issued,
                    publisher: legacy.publisher.map(|n| {
                        Contributor::SimpleName(SimpleName {
                            name: n,
                            location: legacy.publisher_place,
                        })
                    }),
                    url,
                    accessed,
                    note,
                    isbn,
                    doi,
                    edition,
                    keywords: None,
                    original_date: None,
                    original_title: None,
                }))
            }
            "chapter" => {
                let parent_title = legacy
                    .container_title
                    .map(Title::Single)
                    .unwrap_or(Title::Single(String::new()));
                InputReference::CollectionComponent(Box::new(CollectionComponent {
                    id,
                    r#type: MonographComponentType::Chapter,
                    title: Some(title),
                    author: legacy.author.map(Contributor::from),
                    translator: legacy.translator.map(Contributor::from),
                    issued,
                    parent: Parent::Embedded(Collection {
                        id: None,
                        r#type: CollectionType::EditedBook,
                        title: Some(parent_title),
                        editor: legacy.editor.map(Contributor::from),
                        translator: None,
                        issued: EdtfString(String::new()),
                        publisher: legacy.publisher.map(|n| {
                            Contributor::SimpleName(SimpleName {
                                name: n,
                                location: legacy.publisher_place,
                            })
                        }),
                        url: None,
                        accessed: None,
                        note: None,
                        isbn: None,
                        keywords: None,
                    }),
                    pages: legacy.page.map(NumOrStr::Str),
                    url,
                    accessed,
                    note,
                    doi,
                    keywords: None,
                }))
            }
            "article-journal" | "article" | "article-magazine" | "article-newspaper" => {
                let serial_type = match legacy.ref_type.as_str() {
                    "article-journal" => SerialType::AcademicJournal,
                    "article-magazine" => SerialType::Magazine,
                    "article-newspaper" => SerialType::Newspaper,
                    _ => SerialType::AcademicJournal,
                };
                let parent_title = legacy
                    .container_title
                    .map(Title::Single)
                    .unwrap_or(Title::Single(String::new()));
                InputReference::SerialComponent(Box::new(SerialComponent {
                    id,
                    r#type: SerialComponentType::Article,
                    title: Some(title),
                    author: legacy.author.map(Contributor::from),
                    translator: legacy.translator.map(Contributor::from),
                    issued,
                    parent: Parent::Embedded(Serial {
                        r#type: serial_type,
                        title: parent_title,
                        issn: legacy.issn,
                    }),
                    url,
                    accessed,
                    note,
                    doi,
                    pages: legacy.page,
                    volume: legacy.volume.map(|v| match v {
                        csl_legacy::csl_json::StringOrNumber::String(s) => NumOrStr::Str(s),
                        csl_legacy::csl_json::StringOrNumber::Number(n) => NumOrStr::Number(n),
                    }),
                    issue: legacy.issue.map(|v| match v {
                        csl_legacy::csl_json::StringOrNumber::String(s) => NumOrStr::Str(s),
                        csl_legacy::csl_json::StringOrNumber::Number(n) => NumOrStr::Number(n),
                    }),
                    keywords: None,
                }))
            }
            _ => {
                // Fallback to Monograph for unknown types
                InputReference::Monograph(Box::new(Monograph {
                    id,
                    r#type: MonographType::Document,
                    title,
                    author: legacy.author.map(Contributor::from),
                    editor: legacy.editor.map(Contributor::from),
                    translator: legacy.translator.map(Contributor::from),
                    issued,
                    publisher: legacy.publisher.map(|n| {
                        Contributor::SimpleName(SimpleName {
                            name: n,
                            location: legacy.publisher_place,
                        })
                    }),
                    url,
                    accessed,
                    note,
                    isbn,
                    doi,
                    edition,
                    keywords: None,
                    original_date: None,
                    original_title: None,
                }))
            }
        }
    }
}

impl From<csl_legacy::csl_json::DateVariable> for EdtfString {
    fn from(date: csl_legacy::csl_json::DateVariable) -> Self {
        if let Some(literal) = date.literal {
            return EdtfString(literal);
        }
        if let Some(parts) = date.date_parts {
            if let Some(first) = parts.first() {
                let year = first
                    .first()
                    .map(|y| format!("{:04}", y))
                    .unwrap_or_default();
                let month = first
                    .get(1)
                    .map(|m| format!("-{:02}", m))
                    .unwrap_or_default();
                let day = first
                    .get(2)
                    .map(|d| format!("-{:02}", d))
                    .unwrap_or_default();
                return EdtfString(format!("{}{}{}", year, month, day));
            }
        }
        EdtfString(String::new())
    }
}

impl From<Vec<csl_legacy::csl_json::Name>> for Contributor {
    fn from(names: Vec<csl_legacy::csl_json::Name>) -> Self {
        let contributors: Vec<Contributor> = names
            .into_iter()
            .map(|n| {
                if let Some(literal) = n.literal {
                    Contributor::SimpleName(SimpleName {
                        name: literal,
                        location: None,
                    })
                } else {
                    Contributor::StructuredName(StructuredName {
                        given: n.given.unwrap_or_default(),
                        family: n.family.unwrap_or_default(),
                        suffix: n.suffix,
                        dropping_particle: n.dropping_particle,
                        non_dropping_particle: n.non_dropping_particle,
                    })
                }
            })
            .collect();
        Contributor::ContributorList(ContributorList(contributors))
    }
}

impl InputReference {
    /// Create an InputReference from a biblatex Entry.
    pub fn from_biblatex(entry: &Entry) -> Self {
        let id = Some(entry.key.clone());
        let field_str = |key: &str| {
            entry.fields.get(key).map(|f| {
                f.iter()
                    .map(|c| match &c.v {
                        Chunk::Normal(s) | Chunk::Verbatim(s) => s.as_str(),
                        _ => "",
                    })
                    .collect::<String>()
            })
        };

        let title = field_str("title")
            .map(Title::Single)
            .unwrap_or(Title::Single(String::new()));
        let issued = field_str("date")
            .map(EdtfString)
            .unwrap_or(EdtfString(String::new()));
        let publisher = field_str("publisher").map(|p| {
            Contributor::SimpleName(SimpleName {
                name: p,
                location: field_str("location"),
            })
        });

        let author = entry
            .author()
            .ok()
            .map(|p| Contributor::from_biblatex_persons(&p));
        let editor = entry.editors().ok().map(|e| {
            let all_persons: Vec<Person> = e.into_iter().flat_map(|(persons, _)| persons).collect();
            Contributor::from_biblatex_persons(&all_persons)
        });

        match entry.entry_type.to_string().to_lowercase().as_str() {
            "book" | "mvbook" | "collection" | "mvcollection" => {
                InputReference::Monograph(Box::new(Monograph {
                    id,
                    r#type: MonographType::Book,
                    title,
                    author,
                    editor,
                    translator: None,
                    issued,
                    publisher,
                    url: field_str("url").and_then(|u| Url::parse(&u).ok()),
                    accessed: None,
                    note: field_str("note"),
                    isbn: field_str("isbn"),
                    doi: field_str("doi"),
                    edition: field_str("edition"),
                    keywords: None,
                    original_date: None,
                    original_title: None,
                }))
            }
            "inbook" | "incollection" | "inproceedings" => {
                let parent_title = field_str("booktitle")
                    .map(Title::Single)
                    .unwrap_or(Title::Single(String::new()));
                InputReference::CollectionComponent(Box::new(CollectionComponent {
                    id,
                    r#type: MonographComponentType::Chapter,
                    title: Some(title),
                    author,
                    translator: None,
                    issued,
                    parent: Parent::Embedded(Collection {
                        id: None,
                        r#type: CollectionType::EditedBook,
                        title: Some(parent_title),
                        editor,
                        translator: None,
                        issued: EdtfString(String::new()),
                        publisher,
                        url: None,
                        accessed: None,
                        note: None,
                        isbn: None,
                        keywords: None,
                    }),
                    pages: field_str("pages").map(NumOrStr::Str),
                    url: field_str("url").and_then(|u| Url::parse(&u).ok()),
                    accessed: field_str("urldate").map(EdtfString),
                    note: field_str("note"),
                    doi: field_str("doi"),
                    keywords: None,
                }))
            }
            "article" => {
                let parent_title = field_str("journaltitle")
                    .or_else(|| field_str("journal"))
                    .map(Title::Single)
                    .unwrap_or(Title::Single(String::new()));
                InputReference::SerialComponent(Box::new(SerialComponent {
                    id,
                    r#type: SerialComponentType::Article,
                    title: Some(title),
                    author,
                    translator: None,
                    issued,
                    parent: Parent::Embedded(Serial {
                        r#type: SerialType::AcademicJournal, // Default
                        title: parent_title,
                        issn: field_str("issn"),
                    }),
                    url: field_str("url").and_then(|u| Url::parse(&u).ok()),
                    accessed: field_str("urldate").map(EdtfString),
                    note: field_str("note"),
                    doi: field_str("doi"),
                    pages: field_str("pages"),
                    volume: field_str("volume").map(NumOrStr::Str),
                    issue: field_str("number").map(NumOrStr::Str),
                    keywords: None,
                }))
            }
            _ => InputReference::Monograph(Box::new(Monograph {
                id,
                r#type: MonographType::Document,
                title,
                author,
                editor,
                translator: None,
                issued,
                publisher,
                url: field_str("url").and_then(|u| Url::parse(&u).ok()),
                accessed: field_str("urldate").map(EdtfString),
                note: field_str("note"),
                isbn: field_str("isbn"),
                doi: field_str("doi"),
                edition: field_str("edition"),
                keywords: None,
                original_date: None,
                original_title: None,
            })),
        }
    }
}

impl Contributor {
    fn from_biblatex_persons(persons: &[biblatex::Person]) -> Self {
        let contributors: Vec<Contributor> = persons
            .iter()
            .map(|p| {
                Contributor::StructuredName(StructuredName {
                    given: p.given_name.clone(),
                    family: p.name.clone(),
                    suffix: if p.suffix.is_empty() {
                        None
                    } else {
                        Some(p.suffix.clone())
                    },
                    dropping_particle: None,
                    non_dropping_particle: if p.prefix.is_empty() {
                        None
                    } else {
                        Some(p.prefix.clone())
                    },
                })
            })
            .collect();
        Contributor::ContributorList(ContributorList(contributors))
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
    pub editor: Option<Contributor>,
    pub translator: Option<Contributor>,
    pub issued: EdtfString,
    pub publisher: Option<Contributor>,
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub note: Option<String>,
    pub isbn: Option<String>,
    pub doi: Option<String>,
    pub edition: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub original_date: Option<EdtfString>,
    pub original_title: Option<Title>,
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
    pub note: Option<String>,
    pub doi: Option<String>,
    pub pages: Option<String>,
    pub volume: Option<NumOrStr>,
    pub issue: Option<NumOrStr>,
    pub keywords: Option<Vec<String>>,
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
    pub parent: Parent<Collection>,
    pub pages: Option<NumOrStr>,
    pub url: Option<Url>,
    pub accessed: Option<EdtfString>,
    pub note: Option<String>,
    pub doi: Option<String>,
    pub keywords: Option<Vec<String>>,
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

    fn month_to_string(month: u32, months: &[String]) -> String {
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
    pub fn month(&self, months: &[String]) -> String {
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
    pub fn year_month(&self, months: &MonthList) -> String {
        let month = self.month(&months);
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
    pub fn month_day(&self, months: &MonthList) -> String {
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

impl Contributor {
    /// Return the name of the contributor.
    pub fn name(&self) -> Option<String> {
        match self {
            Contributor::SimpleName(n) => Some(n.name.clone()),
            Contributor::StructuredName(n) => Some(format!("{} {}", n.given, n.family)), // Fallback simple formatting
            Contributor::ContributorList(_) => None, // List doesn't have a single name
        }
    }

    /// Return the location of the contributor (mainly for organizations).
    pub fn location(&self) -> Option<String> {
        match self {
            Contributor::SimpleName(n) => n.location.clone(),
            _ => None,
        }
    }
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

/// A flat name structure for processing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlatName {
    pub family: Option<String>,
    pub given: Option<String>,
    pub literal: Option<String>,
    pub suffix: Option<String>,
    pub dropping_particle: Option<String>,
    pub non_dropping_particle: Option<String>,
}

impl FlatName {
    pub fn family_or_literal(&self) -> &str {
        self.family
            .as_deref()
            .or(self.literal.as_deref())
            .unwrap_or("")
    }
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

impl Contributor {
    /// Flatten the contributor into a list of names.
    pub fn to_names_vec(&self) -> Vec<FlatName> {
        match self {
            Contributor::SimpleName(c) => vec![FlatName {
                literal: Some(c.name.clone()),
                ..Default::default()
            }],
            Contributor::StructuredName(c) => vec![FlatName {
                family: Some(c.family.clone()),
                given: Some(c.given.clone()),
                suffix: c.suffix.clone(),
                dropping_particle: c.dropping_particle.clone(),
                non_dropping_particle: c.non_dropping_particle.clone(),
                ..Default::default()
            }],
            Contributor::ContributorList(list) => {
                list.0.iter().flat_map(|c| c.to_names_vec()).collect()
            }
        }
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
        assert_eq!(date.year_month(&months), "January 2020");
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
        assert_eq!(date.month(&months), "January");
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
        assert_eq!(date.month_day(&months), "March 15");

        // Year-month only: returns empty (no day)
        let date_ym = EdtfString("2020-03".to_string());
        assert_eq!(date_ym.month_day(&months), "");

        // Year only: returns empty
        let date_y = EdtfString("2020".to_string());
        assert_eq!(date_y.month_day(&months), "");
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
