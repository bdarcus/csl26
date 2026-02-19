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

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

pub use self::contributor::{Contributor, ContributorList, FlatName, SimpleName, StructuredName};
pub use self::date::EdtfString;
pub use self::types::*;

/// The Reference model.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
    /// A legal case (court decision).
    LegalCase(Box<LegalCase>),
    /// A statute or legislative act.
    Statute(Box<Statute>),
    /// An international treaty or agreement.
    Treaty(Box<Treaty>),
    /// A legislative or administrative hearing.
    Hearing(Box<Hearing>),
    /// An administrative regulation.
    Regulation(Box<Regulation>),
    /// A legal brief or filing.
    Brief(Box<Brief>),
    /// A classic work with standard citation forms.
    Classic(Box<Classic>),
    /// A patent.
    Patent(Box<Patent>),
    /// A research dataset.
    Dataset(Box<Dataset>),
    /// A technical standard or specification.
    Standard(Box<Standard>),
    /// Software or source code.
    Software(Box<Software>),
}

impl InputReference {
    /// Return the reference ID.
    pub fn id(&self) -> Option<RefID> {
        match self {
            InputReference::Monograph(r) => r.id.clone(),
            InputReference::CollectionComponent(r) => r.id.clone(),
            InputReference::SerialComponent(r) => r.id.clone(),
            InputReference::Collection(r) => r.id.clone(),
            InputReference::LegalCase(r) => r.id.clone(),
            InputReference::Statute(r) => r.id.clone(),
            InputReference::Treaty(r) => r.id.clone(),
            InputReference::Hearing(r) => r.id.clone(),
            InputReference::Regulation(r) => r.id.clone(),
            InputReference::Brief(r) => r.id.clone(),
            InputReference::Classic(r) => r.id.clone(),
            InputReference::Patent(r) => r.id.clone(),
            InputReference::Dataset(r) => r.id.clone(),
            InputReference::Standard(r) => r.id.clone(),
            InputReference::Software(r) => r.id.clone(),
        }
    }

    /// Return the author.
    pub fn author(&self) -> Option<Contributor> {
        match self {
            InputReference::Monograph(r) => r.author.clone(),
            InputReference::CollectionComponent(r) => r.author.clone(),
            InputReference::SerialComponent(r) => r.author.clone(),
            InputReference::Treaty(r) => r.author.clone(),
            InputReference::Brief(r) => r.author.clone(),
            InputReference::Classic(r) => r.author.clone(),
            InputReference::Patent(r) => r.author.clone(),
            InputReference::Dataset(r) => r.author.clone(),
            InputReference::Software(r) => r.author.clone(),
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
            InputReference::Classic(r) => r.editor.clone(),
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
            InputReference::Classic(r) => r.translator.clone(),
            _ => None,
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
            InputReference::SerialComponent(r) => {
                let r = r.as_ref();
                match &r.parent {
                    Parent::Embedded(p) => p.publisher.clone(),
                    Parent::Id(_) => None,
                }
            }
            InputReference::Collection(r) => r.publisher.clone(),
            InputReference::Classic(r) => r.publisher.clone(),
            InputReference::Dataset(r) => r.publisher.clone(),
            InputReference::Standard(r) => r.publisher.clone(),
            InputReference::Software(r) => r.publisher.clone(),
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
            InputReference::LegalCase(r) => Some(r.title.clone()),
            InputReference::Statute(r) => Some(r.title.clone()),
            InputReference::Treaty(r) => Some(r.title.clone()),
            InputReference::Hearing(r) => Some(r.title.clone()),
            InputReference::Regulation(r) => Some(r.title.clone()),
            InputReference::Brief(r) => Some(r.title.clone()),
            InputReference::Classic(r) => Some(r.title.clone()),
            InputReference::Patent(r) => Some(r.title.clone()),
            InputReference::Dataset(r) => Some(r.title.clone()),
            InputReference::Standard(r) => Some(r.title.clone()),
            InputReference::Software(r) => Some(r.title.clone()),
        }
    }

    /// Return the issued date.
    pub fn issued(&self) -> Option<EdtfString> {
        match self {
            InputReference::Monograph(r) => Some(r.issued.clone()),
            InputReference::CollectionComponent(r) => Some(r.issued.clone()),
            InputReference::SerialComponent(r) => Some(r.issued.clone()),
            InputReference::Collection(r) => Some(r.issued.clone()),
            InputReference::LegalCase(r) => Some(r.issued.clone()),
            InputReference::Statute(r) => Some(r.issued.clone()),
            InputReference::Treaty(r) => Some(r.issued.clone()),
            InputReference::Hearing(r) => Some(r.issued.clone()),
            InputReference::Regulation(r) => Some(r.issued.clone()),
            InputReference::Brief(r) => Some(r.issued.clone()),
            InputReference::Classic(r) => Some(r.issued.clone()),
            InputReference::Patent(r) => Some(r.issued.clone()),
            InputReference::Dataset(r) => Some(r.issued.clone()),
            InputReference::Standard(r) => Some(r.issued.clone()),
            InputReference::Software(r) => Some(r.issued.clone()),
        }
    }

    /// Return the DOI.
    pub fn doi(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.doi.clone(),
            InputReference::CollectionComponent(r) => r.doi.clone(),
            InputReference::SerialComponent(r) => r.doi.clone(),
            InputReference::LegalCase(r) => r.doi.clone(),
            InputReference::Dataset(r) => r.doi.clone(),
            InputReference::Software(r) => r.doi.clone(),
            _ => None,
        }
    }

    /// Return the note.
    pub fn note(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.note.clone(),
            InputReference::CollectionComponent(r) => r.note.clone(),
            InputReference::SerialComponent(r) => r.note.clone(),
            InputReference::LegalCase(r) => r.note.clone(),
            InputReference::Statute(r) => r.note.clone(),
            InputReference::Treaty(r) => r.note.clone(),
            InputReference::Standard(r) => r.note.clone(),
            _ => None,
        }
    }

    /// Return the URL.
    pub fn url(&self) -> Option<Url> {
        match self {
            InputReference::Monograph(r) => r.url.clone(),
            InputReference::CollectionComponent(r) => r.url.clone(),
            InputReference::SerialComponent(r) => r.url.clone(),
            InputReference::Collection(r) => r.url.clone(),
            InputReference::LegalCase(r) => r.url.clone(),
            InputReference::Statute(r) => r.url.clone(),
            InputReference::Treaty(r) => r.url.clone(),
            InputReference::Hearing(r) => r.url.clone(),
            InputReference::Regulation(r) => r.url.clone(),
            InputReference::Brief(r) => r.url.clone(),
            InputReference::Classic(r) => r.url.clone(),
            InputReference::Patent(r) => r.url.clone(),
            InputReference::Dataset(r) => r.url.clone(),
            InputReference::Standard(r) => r.url.clone(),
            InputReference::Software(r) => r.url.clone(),
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
            InputReference::SerialComponent(r) => match &r.parent {
                Parent::Embedded(p) => p.publisher.as_ref().and_then(|c| c.location()),
                _ => None,
            },
            InputReference::Collection(r) => r.publisher.as_ref().and_then(|c| c.location()),
            InputReference::Classic(r) => r.publisher.as_ref().and_then(|c| c.location()),
            InputReference::Dataset(r) => r.publisher.as_ref().and_then(|c| c.location()),
            InputReference::Standard(r) => r.publisher.as_ref().and_then(|c| c.location()),
            InputReference::Software(r) => r.publisher.as_ref().and_then(|c| c.location()),
            _ => None,
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
            InputReference::SerialComponent(r) => match &r.parent {
                Parent::Embedded(p) => p.publisher.as_ref().and_then(|c| c.name()),
                _ => None,
            },
            InputReference::Collection(r) => r.publisher.as_ref().and_then(|c| c.name()),
            InputReference::Classic(r) => r.publisher.as_ref().and_then(|c| c.name()),
            InputReference::Dataset(r) => r.publisher.as_ref().and_then(|c| c.name()),
            InputReference::Standard(r) => r.publisher.as_ref().and_then(|c| c.name()),
            InputReference::Software(r) => r.publisher.as_ref().and_then(|c| c.name()),
            _ => None,
        }
    }

    /// Return the genre/type as string.
    pub fn genre(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.genre.clone(),
            InputReference::CollectionComponent(r) => r.genre.clone(),
            InputReference::SerialComponent(r) => r.genre.clone(),
            _ => None,
        }
    }

    /// Return the medium.
    pub fn medium(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.medium.clone(),
            InputReference::CollectionComponent(r) => r.medium.clone(),
            InputReference::SerialComponent(r) => r.medium.clone(),
            _ => None,
        }
    }

    /// Return the version.
    pub fn version(&self) -> Option<String> {
        match self {
            InputReference::Dataset(r) => r.version.clone(),
            InputReference::Software(r) => r.version.clone(),
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
            InputReference::LegalCase(r) => r.volume.clone().map(NumOrStr::Str),
            InputReference::Statute(r) => r.volume.clone().map(NumOrStr::Str),
            InputReference::Treaty(r) => r.volume.clone().map(NumOrStr::Str),
            InputReference::Regulation(r) => r.volume.clone().map(NumOrStr::Str),
            InputReference::Classic(r) => r.volume.clone().map(NumOrStr::Str),
            _ => None,
        }
    }

    /// Return the collection number (series number).
    pub fn collection_number(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.collection_number.clone(),
            InputReference::Collection(r) => r.collection_number.clone(),
            InputReference::CollectionComponent(r) => match &r.parent {
                Parent::Embedded(p) => p.collection_number.clone(),
                Parent::Id(_) => None,
            },
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
            InputReference::LegalCase(r) => r.page.clone().map(NumOrStr::Str),
            InputReference::Treaty(r) => r.page.clone().map(NumOrStr::Str),
            _ => None,
        }
    }

    /// Return the authority (court, legislative body, standards org, etc.).
    pub fn authority(&self) -> Option<String> {
        match self {
            InputReference::LegalCase(r) => Some(r.authority.clone()),
            InputReference::Statute(r) => r.authority.clone(),
            InputReference::Hearing(r) => r.authority.clone(),
            InputReference::Regulation(r) => r.authority.clone(),
            InputReference::Brief(r) => r.authority.clone(),
            InputReference::Standard(r) => r.authority.clone(),
            _ => None,
        }
    }

    /// Return the reporter (legal reporter series).
    pub fn reporter(&self) -> Option<String> {
        match self {
            InputReference::LegalCase(r) => r.reporter.clone(),
            InputReference::Treaty(r) => r.reporter.clone(),
            _ => None,
        }
    }

    /// Return the code (legal code abbreviation).
    pub fn code(&self) -> Option<String> {
        match self {
            InputReference::Statute(r) => r.code.clone(),
            InputReference::Regulation(r) => r.code.clone(),
            _ => None,
        }
    }

    /// Return the section (legal section number).
    pub fn section(&self) -> Option<String> {
        match self {
            InputReference::Statute(r) => r.section.clone(),
            InputReference::Regulation(r) => r.section.clone(),
            InputReference::Classic(r) => r.section.clone(),
            _ => None,
        }
    }

    /// Return the number (docket number, session number, etc.).
    pub fn number(&self) -> Option<String> {
        match self {
            InputReference::Monograph(r) => r.report_number.clone(),
            InputReference::Hearing(r) => r.session_number.clone(),
            InputReference::Brief(r) => r.docket_number.clone(),
            InputReference::Patent(r) => Some(r.patent_number.clone()),
            InputReference::Standard(r) => Some(r.standard_number.clone()),
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
            InputReference::LegalCase(r) => r.accessed.clone(),
            InputReference::Statute(r) => r.accessed.clone(),
            InputReference::Treaty(r) => r.accessed.clone(),
            InputReference::Hearing(r) => r.accessed.clone(),
            InputReference::Regulation(r) => r.accessed.clone(),
            InputReference::Brief(r) => r.accessed.clone(),
            InputReference::Classic(r) => r.accessed.clone(),
            InputReference::Patent(r) => r.accessed.clone(),
            InputReference::Dataset(r) => r.accessed.clone(),
            InputReference::Standard(r) => r.accessed.clone(),
            InputReference::Software(r) => r.accessed.clone(),
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
            InputReference::LegalCase(r) => r.keywords.clone(),
            InputReference::Statute(r) => r.keywords.clone(),
            InputReference::Treaty(r) => r.keywords.clone(),
            InputReference::Hearing(r) => r.keywords.clone(),
            InputReference::Regulation(r) => r.keywords.clone(),
            InputReference::Brief(r) => r.keywords.clone(),
            InputReference::Classic(r) => r.keywords.clone(),
            InputReference::Patent(r) => r.keywords.clone(),
            InputReference::Dataset(r) => r.keywords.clone(),
            InputReference::Standard(r) => r.keywords.clone(),
            InputReference::Software(r) => r.keywords.clone(),
        }
    }

    /// Return the language.
    pub fn language(&self) -> Option<LangID> {
        match self {
            InputReference::Monograph(r) => r.language.clone(),
            InputReference::CollectionComponent(r) => r.language.clone(),
            InputReference::SerialComponent(r) => r.language.clone(),
            InputReference::Collection(r) => r.language.clone(),
            InputReference::LegalCase(r) => r.language.clone(),
            InputReference::Statute(r) => r.language.clone(),
            InputReference::Treaty(r) => r.language.clone(),
            InputReference::Hearing(r) => r.language.clone(),
            InputReference::Regulation(r) => r.language.clone(),
            InputReference::Brief(r) => r.language.clone(),
            InputReference::Classic(r) => r.language.clone(),
            InputReference::Patent(r) => r.language.clone(),
            InputReference::Dataset(r) => r.language.clone(),
            InputReference::Standard(r) => r.language.clone(),
            InputReference::Software(r) => r.language.clone(),
        }
    }

    /// Set the reference ID.
    pub fn set_id(&mut self, id: String) {
        match self {
            InputReference::Monograph(monograph) => monograph.id = Some(id),
            InputReference::CollectionComponent(component) => component.id = Some(id),
            InputReference::SerialComponent(component) => component.id = Some(id),
            InputReference::Collection(collection) => collection.id = Some(id),
            InputReference::LegalCase(r) => r.id = Some(id),
            InputReference::Statute(r) => r.id = Some(id),
            InputReference::Treaty(r) => r.id = Some(id),
            InputReference::Hearing(r) => r.id = Some(id),
            InputReference::Regulation(r) => r.id = Some(id),
            InputReference::Brief(r) => r.id = Some(id),
            InputReference::Classic(r) => r.id = Some(id),
            InputReference::Patent(r) => r.id = Some(id),
            InputReference::Dataset(r) => r.id = Some(id),
            InputReference::Standard(r) => r.id = Some(id),
            InputReference::Software(r) => r.id = Some(id),
        }
    }

    /// Return the reference type as a string (CSL-compatible).
    pub fn ref_type(&self) -> String {
        match self {
            InputReference::Monograph(r) => match r.r#type {
                MonographType::Book => {
                    if r.medium
                        .as_deref()
                        .is_some_and(|m| m.to_ascii_lowercase().contains("interview"))
                    {
                        "interview".to_string()
                    } else {
                        "book".to_string()
                    }
                }
                MonographType::Report => "report".to_string(),
                MonographType::Thesis => "thesis".to_string(),
                MonographType::Webpage => "webpage".to_string(),
                MonographType::Post => "post".to_string(),
                MonographType::PersonalCommunication => "personal-communication".to_string(),
                MonographType::Document => {
                    if r.medium
                        .as_deref()
                        .is_some_and(|m| m.to_ascii_lowercase().contains("interview"))
                    {
                        "interview".to_string()
                    } else {
                        "document".to_string()
                    }
                }
            },
            InputReference::CollectionComponent(r) => match r.r#type {
                MonographComponentType::Chapter => "chapter".to_string(),
                MonographComponentType::Document => "paper-conference".to_string(),
            },
            InputReference::SerialComponent(r) => match r.parent {
                Parent::Embedded(ref s) => match s.r#type {
                    SerialType::AcademicJournal => {
                        if r.genre.as_deref() == Some("entry-encyclopedia") {
                            "entry-encyclopedia".to_string()
                        } else {
                            "article-journal".to_string()
                        }
                    }
                    SerialType::Magazine => "article-magazine".to_string(),
                    SerialType::Newspaper => "article-newspaper".to_string(),
                    SerialType::BroadcastProgram => {
                        if r.genre
                            .as_deref()
                            .is_some_and(|g| g.to_ascii_lowercase().contains("film"))
                        {
                            "motion-picture".to_string()
                        } else {
                            "broadcast".to_string()
                        }
                    }
                    _ => "article-journal".to_string(),
                },
                Parent::Id(_) => "article-journal".to_string(),
            },
            InputReference::Collection(r) => match r.r#type {
                CollectionType::EditedBook => "book".to_string(),
                _ => "collection".to_string(),
            },
            InputReference::LegalCase(_) => "legal-case".to_string(),
            InputReference::Statute(_) => "statute".to_string(),
            InputReference::Treaty(_) => "treaty".to_string(),
            InputReference::Hearing(_) => "hearing".to_string(),
            InputReference::Regulation(_) => "regulation".to_string(),
            InputReference::Brief(_) => "brief".to_string(),
            InputReference::Classic(_) => "classic".to_string(),
            InputReference::Patent(_) => "patent".to_string(),
            InputReference::Dataset(_) => "dataset".to_string(),
            InputReference::Standard(_) => "standard".to_string(),
            InputReference::Software(_) => "software".to_string(),
        }
    }
}
