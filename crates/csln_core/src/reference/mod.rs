/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! A reference is a bibliographic item, such as a book, article, or web page.
//! It is the basic unit of bibliographic data.

pub mod contributor;
pub mod conversion;
pub mod date;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod multilingual_tests;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

pub use self::contributor::{Contributor, ContributorList, FlatName, SimpleName, StructuredName};
pub use self::date::EdtfString;
pub use self::types::*;

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
    pub fn id(&self) -> Option<RefID> {
        match self {
            InputReference::Monograph(r) => r.id.clone(),
            InputReference::CollectionComponent(r) => r.id.clone(),
            InputReference::SerialComponent(r) => r.id.clone(),
            InputReference::Collection(r) => r.id.clone(),
        }
    }

    /// Return the author.
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
            InputReference::CollectionComponent(r) => match &r.parent {
                Parent::Embedded(p) => p.editor.clone(),
                Parent::Id(_) => None,
            },
            _ => None,
        }
    }

    /// Return the translator.
    pub fn translator(&self) -> Option<Contributor> {
        match self {
            InputReference::Monograph(r) => r.translator.clone(),
            InputReference::CollectionComponent(r) => r.translator.clone(),
            InputReference::SerialComponent(r) => r.translator.clone(),
            InputReference::Collection(r) => r.translator.clone(),
        }
    }

    /// Return the publisher.
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
    pub fn title(&self) -> Option<Title> {
        match self {
            InputReference::Monograph(r) => Some(r.title.clone()),
            InputReference::CollectionComponent(r) => r.title.clone(),
            InputReference::SerialComponent(r) => r.title.clone(),
            InputReference::Collection(r) => r.title.clone(),
        }
    }

    /// Return the issued date.
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

    /// Return the publisher place.
    pub fn publisher_place(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.publisher.as_ref().and_then(|c| c.location()),
            InputReference::CollectionComponent(r) => match &r.parent {
                Parent::Embedded(p) => p.publisher.as_ref().and_then(|c| c.location()),
                _ => None,
            },
            InputReference::SerialComponent(_) => None,
            InputReference::Collection(r) => r.publisher.as_ref().and_then(|c| c.location()),
        }
    }

    /// Return the publisher as a string.
    pub fn publisher_str(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.publisher.as_ref().and_then(|c| c.name()),
            InputReference::CollectionComponent(r) => match &r.parent {
                Parent::Embedded(p) => p.publisher.as_ref().and_then(|c| c.name()),
                _ => None,
            },
            InputReference::SerialComponent(_) => None,
            InputReference::Collection(r) => r.publisher.as_ref().and_then(|c| c.name()),
        }
    }

    /// Return the genre/type as string.
    pub fn genre(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.genre.clone(),
            InputReference::CollectionComponent(r) => r.genre.clone(),
            _ => None,
        }
    }

    /// Return the abstract.
    pub fn abstract_text(&self) -> Option<String> {
        None
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

    /// Return the language.
    pub fn language(&self) -> Option<LangID> {
        match self {
            InputReference::Monograph(r) => r.language.clone(),
            InputReference::CollectionComponent(r) => r.language.clone(),
            InputReference::SerialComponent(r) => r.language.clone(),
            InputReference::Collection(r) => r.language.clone(),
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
            InputReference::Monograph(r) => match r.r#type {
                MonographType::Book => "book".to_string(),
                MonographType::Report => "report".to_string(),
                MonographType::Thesis => "thesis".to_string(),
                MonographType::Webpage => "webpage".to_string(),
                MonographType::Post => "post".to_string(),
                MonographType::Document => "document".to_string(),
            },
            InputReference::CollectionComponent(r) => match r.r#type {
                MonographComponentType::Chapter => "chapter".to_string(),
                MonographComponentType::Document => "paper-conference".to_string(),
            },
            InputReference::SerialComponent(r) => match r.parent {
                Parent::Embedded(ref s) => match s.r#type {
                    SerialType::AcademicJournal => "article-journal".to_string(),
                    SerialType::Magazine => "article-magazine".to_string(),
                    SerialType::Newspaper => "article-newspaper".to_string(),
                    _ => "article-journal".to_string(),
                },
                Parent::Id(_) => "article-journal".to_string(),
            },
            InputReference::Collection(r) => match r.r#type {
                CollectionType::EditedBook => "book".to_string(),
                _ => "collection".to_string(),
            },
        }
    }
}
