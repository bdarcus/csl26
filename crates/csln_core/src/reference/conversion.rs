use crate::reference::contributor::{Contributor, ContributorList, SimpleName, StructuredName};
use crate::reference::date::EdtfString;
use crate::reference::types::*;
use crate::reference::InputReference;
use biblatex::{Chunk, Entry, Person};
use url::Url;

impl From<csl_legacy::csl_json::Reference> for InputReference {
    fn from(legacy: csl_legacy::csl_json::Reference) -> Self {
        let id = Some(legacy.id);
        let language = legacy.language;
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
            "book" | "report" | "thesis" | "webpage" | "post" | "post-weblog" | "software"
            | "dataset" => {
                let r#type = if legacy.ref_type == "report" {
                    MonographType::Report
                } else if legacy.ref_type == "thesis" {
                    MonographType::Thesis
                } else if legacy.ref_type == "webpage" {
                    MonographType::Webpage
                } else if legacy.ref_type.contains("post") {
                    MonographType::Post
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
                            name: n.into(),
                            location: legacy.publisher_place,
                        })
                    }),
                    url,
                    accessed,
                    language,
                    note,
                    isbn,
                    doi,
                    edition,
                    genre: legacy.genre,
                    keywords: None,
                    original_date: None,
                    original_title: None,
                }))
            }
            "chapter" | "paper-conference" => {
                let parent_title = legacy
                    .container_title
                    .map(Title::Single)
                    .unwrap_or(Title::Single(String::new()));
                InputReference::CollectionComponent(Box::new(CollectionComponent {
                    id,
                    r#type: if legacy.ref_type == "paper-conference" {
                        MonographComponentType::Document
                    } else {
                        MonographComponentType::Chapter
                    },
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
                                name: n.into(),
                                location: legacy.publisher_place,
                            })
                        }),
                        url: None,
                        accessed: None,
                        language: None,
                        note: None,
                        isbn: None,
                        keywords: None,
                    }),
                    pages: legacy.page.map(NumOrStr::Str),
                    url,
                    accessed,
                    language,
                    note,
                    doi,
                    genre: legacy.genre,
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
                    language,
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
            _ => InputReference::Monograph(Box::new(Monograph {
                id,
                r#type: MonographType::Document,
                title,
                author: legacy.author.map(Contributor::from),
                editor: legacy.editor.map(Contributor::from),
                translator: legacy.translator.map(Contributor::from),
                issued,
                publisher: legacy.publisher.map(|n| {
                    Contributor::SimpleName(SimpleName {
                        name: n.into(),
                        location: legacy.publisher_place,
                    })
                }),
                url,
                accessed,
                language,
                note,
                isbn,
                doi,
                edition,
                genre: legacy.genre,
                keywords: None,
                original_date: None,
                original_title: None,
            })),
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
                        name: literal.into(),
                        location: None,
                    })
                } else {
                    Contributor::StructuredName(StructuredName {
                        given: n.given.unwrap_or_default().into(),
                        family: n.family.unwrap_or_default().into(),
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
                name: p.into(),
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

        let language = field_str("langid").or_else(|| field_str("language"));

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
                    language,
                    note: field_str("note"),
                    isbn: field_str("isbn"),
                    doi: field_str("doi"),
                    edition: field_str("edition"),
                    genre: field_str("type"),
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
                        language: None,
                        note: None,
                        isbn: None,
                        keywords: None,
                    }),
                    pages: field_str("pages").map(NumOrStr::Str),
                    url: field_str("url").and_then(|u| Url::parse(&u).ok()),
                    accessed: field_str("urldate").map(EdtfString),
                    language,
                    note: field_str("note"),
                    doi: field_str("doi"),
                    genre: field_str("type"),
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
                        r#type: SerialType::AcademicJournal,
                        title: parent_title,
                        issn: field_str("issn"),
                    }),
                    url: field_str("url").and_then(|u| Url::parse(&u).ok()),
                    accessed: field_str("urldate").map(EdtfString),
                    language,
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
                language,
                note: field_str("note"),
                isbn: field_str("isbn"),
                doi: field_str("doi"),
                edition: field_str("edition"),
                genre: field_str("type"),
                keywords: None,
                original_date: None,
                original_title: None,
            })),
        }
    }
}

impl Contributor {
    fn from_biblatex_persons(persons: &[Person]) -> Self {
        let contributors: Vec<Contributor> = persons
            .iter()
            .map(|p| {
                Contributor::StructuredName(StructuredName {
                    given: p.given_name.clone().into(),
                    family: p.name.clone().into(),
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
