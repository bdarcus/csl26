/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Biblatex entry conversion to Citum `InputReference`.
//!
//! Provides functions to convert biblatex entries and contributor
//! information into Citum's `InputReference` and Contributor types.

use biblatex as biblatex_crate;
use biblatex_crate::ChunksExt as _;
use citum_schema::reference::{
    InputReference, LangID, Numbering, NumberingType, Publisher, RefID, RichText, WorkRelation,
    citeproc_markup::html_markup_to_djot,
    contributor::{
        Contributor, ContributorEntry, ContributorList, ContributorRole, SimpleName, StructuredName,
    },
    date::DateValue,
    types::{
        Collection, CollectionComponent, CollectionType, Dataset, EprintInfo, Event, Monograph,
        MonographComponentType, MonographType, NumOrStr, Patent, Serial, SerialComponent,
        SerialComponentType, SerialType, Software, Standard, StructuredTitle, Subtitle, Title,
    },
};
use std::collections::HashMap;
use url::Url;

use super::tables::{BiblatexBuilder, biblatex_entry_mapping};

/// Common fields shared across all biblatex reference conversion helpers.
struct BibRefContext<'a> {
    id: Option<RefID>,
    title: Option<Title>,
    author: Option<Contributor>,
    editor: Option<Contributor>,
    translator: Option<Contributor>,
    /// Contributors that don't have a shorthand slot on `InputReference`:
    /// non-`Editor`-typed `editora`/`editorb`/`editorc` groups (compiler,
    /// director, ...) and the editorial sub-role fields (`annotator`,
    /// `commentator`, `foreword`, `introduction`, `afterword`). Folded into
    /// the canonical `contributors` vec by `normalize_contributors()`
    /// alongside the `author`/`editor`/`translator` shorthands above.
    contributors: Vec<ContributorEntry>,
    issued: DateValue,
    publisher: Option<Publisher>,
    language: Option<LangID>,
    /// Preprint-server identifier from `eprint`/`eprinttype`/`eprintclass`.
    /// Set unconditionally when present; unlike the `MonographType::Preprint`
    /// flip (decided in `input_reference_from_biblatex`, before a builder is
    /// chosen), storing this metadata doesn't depend on the entry's type.
    eprint: Option<EprintInfo>,
    /// Patent holder (biblatex `holder`), for `Patent.assignee`. Only
    /// meaningful on `@patent`; unread by every other builder.
    assignee: Option<Contributor>,
    field_str: &'a dyn Fn(&str) -> Option<String>,
    /// Like `field_str`, but converts citeproc-js's literal HTML rich-text
    /// convention (`<span class="nocase">`, `<i>`, `<b>`, `<sc>`, `<sup>`,
    /// `<sub>`) to Djot. Zotero's builtin BibTeX/BibLaTeX exporter escapes
    /// these as `{\textless}span class="nocase"{\textgreater}…`, which the
    /// `biblatex` parser unescapes back to literal HTML before Citum sees
    /// it (`csl26-6eoi`) -- so free-text fields need the same conversion
    /// the CSL-JSON path applies, not just `field_str`.
    rich_field_str: &'a dyn Fn(&str) -> Option<String>,
    /// Like `rich_field_str`, but for BibLaTeX `LiteralList`-datatype fields
    /// (`publisher`, `institution`, `organization`, `school`, `location`) --
    /// see `literal_list_str`.
    /// Joins multi-entity values with `"; "` instead of leaking the literal
    /// `and` separator.
    rich_literal_list_str: &'a dyn Fn(&str) -> Option<String>,
}

/// Build a `CollectionComponent` from a biblatex inbook/incollection/inproceedings entry.
fn build_inbook_reference(entry_type: &str, ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    let rich_field_str = ctx.rich_field_str;
    let contributors = ctx.contributors;
    let parent_title = rich_field_str("booktitle").map(Title::Single);
    let is_inproceedings = entry_type == "inproceedings";
    let component_type = if is_inproceedings {
        MonographComponentType::Document
    } else {
        MonographComponentType::Chapter
    };

    let mut parent_numbering = Vec::new();
    if let Some(n) = field_str("number") {
        parent_numbering.push(Numbering {
            r#type: NumberingType::Volume,
            value: n,
        });
    }

    let mut numbering = Vec::new();
    if let Some(c) = field_str("chapter") {
        numbering.push(Numbering {
            r#type: NumberingType::Chapter,
            value: c,
        });
    }

    // `eventtitle`/`venue`/`eventdate` name the conference itself, distinct
    // from `booktitle` (the proceedings volume) -- only meaningful for
    // `@inproceedings`. Mirrors `relation_event` in citum-schema-data's
    // CSL-JSON `paper-conference` conversion path.
    let event = is_inproceedings
        .then(|| {
            let title = rich_field_str("eventtitle");
            let location = rich_field_str("venue");
            let date = field_str("eventdate").map(DateValue::new);
            (title.is_some() || location.is_some() || date.is_some()).then(|| {
                WorkRelation::Embedded(Box::new(InputReference::Event(Box::new(Event {
                    title: title.map(Title::Single),
                    location,
                    date,
                    ..Default::default()
                }))))
            })
        })
        .flatten();

    InputReference::CollectionComponent(Box::new(CollectionComponent {
        id: ctx.id,
        r#type: component_type,
        title: ctx.title,
        author: ctx.author,
        // The chapter's translator belongs to the chapter, not the edited
        // volume: `InputReference::translator()` has no container fallback
        // (unlike `publisher()`), so a parent-only translator would be
        // unreachable. See bean csl26-7ab8.
        translator: ctx.translator,
        contributors,
        eprint: ctx.eprint,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        container: Some(WorkRelation::Embedded(Box::new(
            InputReference::Collection(Box::new(Collection {
                id: None,
                r#type: if is_inproceedings {
                    CollectionType::Proceedings
                } else {
                    CollectionType::EditedBook
                },
                title: parent_title,
                short_title: None,
                container: series_relation(field_str("series")),
                editor: ctx.editor,
                translator: None,
                created: DateValue::new(String::new()),
                issued: DateValue::new(String::new()),
                publisher: ctx.publisher,
                numbering: parent_numbering,
                isbn: field_str("isbn"),
                event,
                ..Default::default()
            })),
        ))),
        numbering,
        pages: field_str("pages").map(NumOrStr::Str),
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        doi: field_str("doi"),
        genre: if entry_type == "inreference" {
            Some("entry".to_string())
        } else {
            rich_field_str("type")
        },
        ..Default::default()
    }))
}

/// Build a `SerialComponent` from a biblatex article entry.
fn build_article_reference(ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    let rich_field_str = ctx.rich_field_str;
    let contributors = ctx.contributors;
    let parent_title = rich_field_str("journaltitle")
        .or_else(|| rich_field_str("journal"))
        .map(Title::Single);

    let mut component_numbering = Vec::new();
    if let Some(v) = field_str("volume") {
        component_numbering.push(Numbering {
            r#type: NumberingType::Volume,
            value: v,
        });
    }
    if let Some(i) = field_str("number") {
        component_numbering.push(Numbering {
            r#type: NumberingType::Issue,
            value: i,
        });
    }

    InputReference::SerialComponent(Box::new(SerialComponent {
        id: ctx.id,
        r#type: SerialComponentType::Article,
        title: ctx.title,
        author: ctx.author,
        translator: ctx.translator,
        contributors,
        eprint: ctx.eprint,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        container: Some(WorkRelation::Embedded(Box::new(InputReference::Serial(
            Box::new(Serial {
                id: None,
                r#type: SerialType::AcademicJournal,
                title: parent_title,
                short_title: None,
                container: series_relation(field_str("series")),
                // Journal/issue editor: `editor()` falls back to the container
                // for `SerialComponent`, so storing it here is both correct
                // (biblatex `editor` on `@article` names the journal editor)
                // and reachable. See bean csl26-7ab8.
                editor: ctx.editor,
                contributors: Vec::new(),
                publisher: None,
                url: None,
                accessed: None,
                language: None,
                field_languages: HashMap::new(),
                note: None,
                issn: field_str("issn"),
                unknown_fields: Default::default(),
            }),
        )))),
        numbering: component_numbering,
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        doi: field_str("doi"),
        ads_bibcode: field_str("bibcode"),
        pages: field_str("pages"),
        genre: rich_field_str("type"),
        ..Default::default()
    }))
}

/// Build a standalone `Collection` from a biblatex `@collection`/
/// `@mvcollection`/`@proceedings`/`@mvproceedings` entry.
///
/// Unlike `build_inbook_reference`/`build_article_reference`, there is no
/// component embedded in a synthesized parent -- the entry itself *is* the
/// `Collection` (it has no per-chapter structure of its own).
fn build_collection_reference(r#type: CollectionType, ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    let rich_field_str = ctx.rich_field_str;
    let contributors = ctx.contributors;
    let is_proceedings = matches!(r#type, CollectionType::Proceedings);
    let series = field_str("series");

    let mut numbering = Vec::new();
    if let Some(ed) = field_str("edition") {
        numbering.push(Numbering {
            r#type: NumberingType::Edition,
            value: ed,
        });
    }
    if let Some(n) = field_str("number") {
        let numbering_type = if series.is_some() {
            NumberingType::Volume
        } else {
            NumberingType::Number
        };
        numbering.push(Numbering {
            r#type: numbering_type,
            value: n,
        });
    }

    // See `build_inbook_reference` for the same event-field rationale;
    // `Collection` carries `event` directly rather than needing a synthesized
    // parent, since this reference *is* the proceedings volume.
    let event = is_proceedings
        .then(|| {
            let title = rich_field_str("eventtitle");
            let location = rich_field_str("venue");
            let date = field_str("eventdate").map(DateValue::new);
            (title.is_some() || location.is_some() || date.is_some()).then(|| {
                WorkRelation::Embedded(Box::new(InputReference::Event(Box::new(Event {
                    title: title.map(Title::Single),
                    location,
                    date,
                    ..Default::default()
                }))))
            })
        })
        .flatten();

    InputReference::Collection(Box::new(Collection {
        id: ctx.id,
        r#type,
        title: ctx.title,
        short_title: None,
        container: series_relation(series),
        editor: ctx.editor,
        translator: ctx.translator,
        contributors,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        publisher: ctx.publisher,
        numbering,
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        isbn: field_str("isbn"),
        event,
        keywords: split_keywords(field_str("keywords")),
        ..Default::default()
    }))
}

/// Split biblatex's comma-separated `keywords` field into a keyword list,
/// trimming whitespace and dropping empty entries.
fn split_keywords(raw: Option<String>) -> Option<Vec<String>> {
    raw.map(|k| {
        k.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

/// Convert a single biblatex chunk to its string content. `Chunk::Math` is
/// wrapped as Djot inline math (`$...$`) rather than discarded to an empty
/// string -- the `biblatex` crate already strips the delimiting `$...$` from
/// the source when parsing, so this puts them back in Citum's own
/// inline-math convention.
fn chunk_to_string(chunk: &biblatex_crate::Chunk) -> String {
    match chunk {
        biblatex_crate::Chunk::Normal(s) | biblatex_crate::Chunk::Verbatim(s) => s.clone(),
        biblatex_crate::Chunk::Math(s) => format!("${s}$"),
    }
}

/// Concatenate a chunk sequence into a single string via [`chunk_to_string`].
fn chunks_to_string(chunks: &[biblatex_crate::Spanned<biblatex_crate::Chunk>]) -> String {
    chunks.iter().map(|c| chunk_to_string(&c.v)).collect()
}

/// Read a BibLaTeX `LiteralList`-datatype field (`publisher`, `institution`,
/// `organization`, `school`, `location`), which the manual's §2.2.1 allows
/// to name more than one entity separated by `and`. Joins the parsed items with `"; "` rather than
/// concatenating the whole field verbatim, so `location = {Boston} and
/// {London}` becomes `"Boston; London"` instead of leaking the literal
/// `"and"` separator into the string. Every other field stays on plain
/// concatenation (`field_str`/`rich_field_str`) -- this is deliberately
/// narrow, not a general typed-field rewrite; see the module doc.
fn literal_list_str(entry: &biblatex_crate::Entry, key: &str) -> Option<String> {
    let chunks = entry.get(key)?;
    let items = chunks.parse::<Vec<biblatex_crate::Chunks>>().ok()?;
    Some(
        items
            .iter()
            .map(|item| chunks_to_string(item))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Build a standalone `Patent` from a biblatex `@patent` entry. `number` is
/// required by `Patent.patent_number` (non-`Option`) -- callers must confirm
/// it is present first (see the dispatch guard in
/// `input_reference_from_biblatex`).
fn build_patent_reference(patent_number: String, ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    let rich_literal_list_str = ctx.rich_literal_list_str;
    InputReference::Patent(Box::new(Patent {
        id: ctx.id,
        title: ctx.title,
        author: ctx.author,
        assignee: ctx.assignee,
        original: None,
        patent_number,
        application_number: None,
        pages: field_str("pages"),
        created: DateValue::new(String::new()),
        filing_date: None,
        issued: ctx.issued,
        jurisdiction: rich_literal_list_str("location"),
        authority: rich_literal_list_str("organization")
            .or_else(|| rich_literal_list_str("institution")),
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        keywords: split_keywords(field_str("keywords")),
        unknown_fields: Default::default(),
    }))
}

/// Build a standalone `Dataset` from a biblatex `@dataset` entry.
fn build_dataset_reference(ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    let rich_field_str = ctx.rich_field_str;
    InputReference::Dataset(Box::new(Dataset {
        id: ctx.id,
        title: ctx.title,
        author: ctx.author,
        original: None,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        publisher: ctx.publisher,
        version: field_str("version"),
        genre: rich_field_str("type"),
        format: None,
        size: None,
        repository: None,
        doi: field_str("doi"),
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        keywords: split_keywords(field_str("keywords")),
        unknown_fields: Default::default(),
    }))
}

/// Build a standalone `Software` from a biblatex `@software` entry.
fn build_software_reference(ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    InputReference::Software(Box::new(Software {
        id: ctx.id,
        title: ctx.title,
        original: None,
        author: ctx.author,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        publisher: ctx.publisher,
        version: field_str("version"),
        repository: None,
        license: None,
        platform: None,
        doi: field_str("doi"),
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        keywords: split_keywords(field_str("keywords")),
        unknown_fields: Default::default(),
    }))
}

/// Build a standalone `Standard` from a biblatex `@standard` entry (not a
/// core biblatex type; arrives as `EntryType::Unknown("standard")`).
/// `standard_number` is required (non-`Option`) -- callers must confirm it
/// is present first (see the dispatch guard in
/// `input_reference_from_biblatex`).
fn build_standard_reference(standard_number: String, ctx: BibRefContext<'_>) -> InputReference {
    let field_str = ctx.field_str;
    let rich_literal_list_str = ctx.rich_literal_list_str;
    InputReference::Standard(Box::new(Standard {
        id: ctx.id,
        title: ctx.title,
        original: None,
        authority: rich_literal_list_str("organization")
            .or_else(|| rich_literal_list_str("institution")),
        standard_number,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        status: None,
        publisher: ctx.publisher,
        doi: field_str("doi"),
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        keywords: split_keywords(field_str("keywords")),
        unknown_fields: Default::default(),
    }))
}

/// Resolve the lowercased, alias-resolved BibLaTeX entry-type string used to
/// look up a row in `BIBLATEX_ENTRY_TYPES`.
///
/// `EntryType::to_string()` discards the payload of `Unknown(_)` -- it
/// prints the literal string `"unknown"`, not the source's actual type name
/// -- and `.to_biblatex()` additionally collapses every `Unknown` variant to
/// `Misc`. Both would make any non-core vocabulary (e.g. the GB/T
/// 7714-style `@standard`) permanently unreachable in `BIBLATEX_ENTRY_TYPES`,
/// silently landing on the fallback under the wrong identity. Read the
/// original string directly for `Unknown` types; canonical types (including
/// the phdthesis/mastersthesis/techreport aliases) still go through
/// `to_biblatex()` as before.
fn biblatex_entry_type_key(entry: &biblatex_crate::Entry) -> String {
    match &entry.entry_type {
        biblatex_crate::EntryType::Unknown(raw) => raw.to_lowercase(),
        other => other.to_biblatex().to_string().to_lowercase(),
    }
}

/// Dispatch to the builder named by `biblatex_entry_mapping(entry_type)`,
/// applying the two type-flip overrides that depend on entry-level state the
/// static table can't express: the container-less-`@article` and
/// carries-an-`eprint` preprint rules, and the `Patent`/`Standard`
/// nonblank-required-field fallback.
fn dispatch_biblatex_builder(
    entry_type: &str,
    has_eprint: bool,
    article_is_container_less: bool,
    ctx: BibRefContext<'_>,
) -> InputReference {
    if article_is_container_less {
        return InputReference::Monograph(Box::new(biblatex_monograph(
            MonographType::Preprint,
            entry_type,
            ctx,
        )));
    }
    let field_str = ctx.field_str;
    match &biblatex_entry_mapping(entry_type).builder {
        BiblatexBuilder::Monograph(mono_type) => {
            // A `misc`/`unpublished`/`online`/fallback entry carrying an
            // `eprint` is a preprint, not a generic document/manuscript/
            // webpage. Doesn't apply to types with more specific semantics
            // (`@book`, `@thesis`, ...) -- a stray `eprint` there doesn't
            // override the entry-type-driven mapping.
            let mono_type = if has_eprint && is_document_like_monograph_type(mono_type) {
                MonographType::Preprint
            } else {
                mono_type.clone()
            };
            InputReference::Monograph(Box::new(biblatex_monograph(mono_type, entry_type, ctx)))
        }
        BiblatexBuilder::Inbook => build_inbook_reference(entry_type, ctx),
        BiblatexBuilder::Article => build_article_reference(ctx),
        BiblatexBuilder::Collection(collection_type) => {
            build_collection_reference(collection_type.clone(), ctx)
        }
        // `Patent.patent_number`/`Standard.standard_number` are required
        // (non-`Option`) fields; an entry with no nonblank `number` stays on
        // the generic fallback rather than being given an empty identifier.
        BiblatexBuilder::Patent => {
            match field_str("number").filter(|number| !number.trim().is_empty()) {
                Some(number) => build_patent_reference(number, ctx),
                None => InputReference::Monograph(Box::new(biblatex_monograph(
                    MonographType::Document,
                    entry_type,
                    ctx,
                ))),
            }
        }
        BiblatexBuilder::Dataset => build_dataset_reference(ctx),
        BiblatexBuilder::Software => build_software_reference(ctx),
        BiblatexBuilder::Standard => {
            match field_str("number").filter(|number| !number.trim().is_empty()) {
                Some(number) => build_standard_reference(number, ctx),
                None => InputReference::Monograph(Box::new(biblatex_monograph(
                    MonographType::Document,
                    entry_type,
                    ctx,
                ))),
            }
        }
    }
}

/// Convert a biblatex entry into an `InputReference`.
///
/// Maps biblatex entry types (book, article, inproceedings, etc.) to
/// appropriate Citum reference types. Extracts all relevant fields
/// including contributors, dates, and metadata.
pub fn input_reference_from_biblatex(entry: &biblatex_crate::Entry) -> InputReference {
    let id = Some(entry.key.clone().into());
    let field_str = |key: &str| entry.fields.get(key).map(|f| chunks_to_string(f));
    let rich_field_str = |key: &str| field_str(key).map(|s| html_markup_to_djot(&s));
    let rich_literal_list_str =
        |key: &str| literal_list_str(entry, key).map(|s| html_markup_to_djot(&s));

    let title = match (rich_field_str("title"), rich_field_str("subtitle")) {
        (Some(main), Some(sub)) => Some(Title::Structured(StructuredTitle {
            full: None,
            main,
            sub: Subtitle::String(sub),
        })),
        (Some(main), None) => Some(Title::Single(main)),
        (None, _) => None,
    };
    let issued = field_str("date").map_or(DateValue::new(String::new()), DateValue::new);
    let publisher = rich_literal_list_str("publisher")
        .or_else(|| rich_literal_list_str("institution"))
        .or_else(|| rich_literal_list_str("organization"))
        .or_else(|| rich_literal_list_str("school"))
        .map(|p| Publisher {
            name: p.into(),
            place: rich_literal_list_str("location").map(Into::into),
        });

    let author = entry
        .author()
        .ok()
        .map(|p| contributors_from_biblatex_persons(&p));

    // Contributors with no shorthand slot on `InputReference`: non-`Editor`
    // editorial-role groups (`editora`/`editorb`/`editorc` tagged `compiler`,
    // `director`, etc.) and the sub-role name fields. Folded into the
    // canonical `contributors` vec by `normalize_contributors()`.
    let (editor, mut contributors) = editor_groups_from_biblatex(entry);
    let translator = entry
        .translator()
        .ok()
        .map(|p| contributors_from_biblatex_persons(&p));

    contributors.extend(editorial_sub_role_contributors(entry));

    let language = field_str("langid")
        .or_else(|| field_str("language"))
        .map(Into::into);
    let eprint = eprint_info_from_biblatex(&field_str);
    let has_eprint = eprint.is_some();
    let assignee = entry
        .holder()
        .ok()
        .map(|p| contributors_from_biblatex_persons(&p));

    let entry_type = biblatex_entry_type_key(entry);
    // An `@article` with no `journaltitle`/`journal` *and* an `eprint` is
    // treated as a standalone preprint rather than a truncated journal
    // article -- loosely mirroring `CSL_TYPE_MAP`'s rule for a bare CSL-JSON
    // `article`, but requiring `eprint` too: unlike CSL-JSON's generic
    // `article` type, BibLaTeX's `@article` inherently implies journal
    // fields, so a container-less `@article` with no `eprint` either is more
    // likely an incomplete entry than a preprint (a plain `@article{key,
    // author = {...}, title = {...}}` fixture must not become one).
    let article_is_container_less = entry_type == "article"
        && has_eprint
        && field_str("journaltitle").is_none()
        && field_str("journal").is_none();

    let ctx = BibRefContext {
        id,
        title,
        author,
        editor,
        translator,
        contributors,
        issued,
        publisher,
        language,
        eprint,
        assignee,
        field_str: &field_str,
        rich_field_str: &rich_field_str,
        rich_literal_list_str: &rich_literal_list_str,
    };

    let mut reference =
        dispatch_biblatex_builder(&entry_type, has_eprint, article_is_container_less, ctx);
    // The builders above construct references directly rather than through
    // deserialization, so the `author`/`editor`/`translator` shorthands they set
    // are never folded into the canonical `contributors` vec. That vec is the
    // only field serialization preserves — without this call, contributors
    // silently vanish on write (bean csl26-7ab8).
    reference.normalize_contributors();
    reference
}

/// Push a `ContributorEntry` for a biblatex name-list field that has no
/// shorthand slot on `InputReference` — the editorial sub-role fields
/// (`annotator`, `commentator`, `foreword`, `introduction`, `afterword`) and
/// non-primary `editora`/`editorb`/`editorc` `EditorType` groups. No-ops when
/// the field is absent or the person list is empty.
fn push_person_role(
    contributors: &mut Vec<ContributorEntry>,
    persons: Option<Vec<biblatex_crate::Person>>,
    role: ContributorRole,
) {
    let Some(persons) = persons.filter(|p| !p.is_empty()) else {
        return;
    };
    contributors.push(ContributorEntry {
        roles: role.into(),
        contributor: contributors_from_biblatex_persons(&persons),
        gender: None,
    });
}

/// Map a biblatex `EditorType` (the `editortype`/`editoratype`/`editorbtype`/
/// `editorctype` annotation on the `editor`/`editora`/`editorb`/`editorc`
/// fields) to a Citum contributor role. `Founder`/`Continuator`/`Redactor`/
/// `Reviser`/`Collaborator`/`Organizer` have no dedicated `ContributorRole`
/// variant and degrade to `Unknown(<name>)`, which still round-trips and
/// remains selectable by a style as a custom role.
fn contributor_role_for_editor_type(editor_type: &biblatex_crate::EditorType) -> ContributorRole {
    use biblatex_crate::EditorType;
    match editor_type {
        EditorType::Editor => ContributorRole::Editor,
        EditorType::Compiler => ContributorRole::Compiler,
        EditorType::Director => ContributorRole::Director,
        EditorType::Founder => ContributorRole::Unknown("founder".to_string()),
        EditorType::Continuator => ContributorRole::Unknown("continuator".to_string()),
        EditorType::Redactor => ContributorRole::Unknown("redactor".to_string()),
        EditorType::Reviser => ContributorRole::Unknown("reviser".to_string()),
        EditorType::Collaborator => ContributorRole::Unknown("collaborator".to_string()),
        EditorType::Organizer => ContributorRole::Unknown("organizer".to_string()),
        EditorType::Unknown(s) => ContributorRole::Unknown(s.clone()),
    }
}

/// Split biblatex's `editor`/`editora`/`editorb`/`editorc` groups into the
/// `editor` shorthand (the `EditorType::Editor`-tagged groups) and any
/// other-typed groups, returned as `ContributorEntry` values ready to fold
/// into a reference's `contributors` vec. Previously all four groups were
/// flattened into `editor` regardless of `EditorType`, discarding it.
fn editor_groups_from_biblatex(
    entry: &biblatex_crate::Entry,
) -> (Option<Contributor>, Vec<ContributorEntry>) {
    let mut contributors = Vec::new();
    let editor = entry.editors().ok().and_then(|groups| {
        let mut editor_persons: Vec<biblatex_crate::Person> = Vec::new();
        for (persons, editor_type) in groups {
            if matches!(editor_type, biblatex_crate::EditorType::Editor) {
                editor_persons.extend(persons);
            } else {
                push_person_role(
                    &mut contributors,
                    Some(persons),
                    contributor_role_for_editor_type(&editor_type),
                );
            }
        }
        // `entry.editors()` returns `Ok(vec![])` rather than `Err` when the
        // entry has no editor* field at all, so an empty person list must be
        // treated as "no editor" — otherwise a bogus `contributor: []` leaks
        // into every reference (see bean csl26-7ab8).
        if editor_persons.is_empty() {
            None
        } else {
            Some(contributors_from_biblatex_persons(&editor_persons))
        }
    });
    (editor, contributors)
}

/// Read the editorial sub-role name fields (`annotator`, `commentator`,
/// `foreword`, `introduction`, `afterword`) as `ContributorEntry` values.
fn editorial_sub_role_contributors(entry: &biblatex_crate::Entry) -> Vec<ContributorEntry> {
    let mut contributors = Vec::new();
    push_person_role(
        &mut contributors,
        entry.annotator().ok(),
        ContributorRole::Annotator,
    );
    push_person_role(
        &mut contributors,
        entry.commentator().ok(),
        ContributorRole::Commentator,
    );
    push_person_role(
        &mut contributors,
        entry.foreword().ok(),
        ContributorRole::ForewordAuthor,
    );
    push_person_role(
        &mut contributors,
        entry.introduction().ok(),
        ContributorRole::IntroductionAuthor,
    );
    push_person_role(
        &mut contributors,
        entry.afterword().ok(),
        ContributorRole::AfterwordAuthor,
    );
    contributors
}

/// Build the shared `EprintInfo` (preprint-server identifier) from biblatex's
/// `eprint`/`eprinttype` (alias `archiveprefix`)/`eprintclass` (alias
/// `primaryclass`) fields. `server` is lowercased per `EprintInfo::server`'s
/// doc comment; producers may supply mixed case (`"arXiv"`).
fn eprint_info_from_biblatex(field_str: &dyn Fn(&str) -> Option<String>) -> Option<EprintInfo> {
    let id = field_str("eprint").filter(|id| !id.trim().is_empty())?;
    let server = field_str("eprinttype")
        .or_else(|| field_str("archiveprefix"))
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let class = field_str("eprintclass")
        .or_else(|| field_str("primaryclass"))
        .filter(|class| !class.trim().is_empty());
    Some(EprintInfo { id, server, class })
}

/// Whether `t` is generic enough that an `eprint` field should override it to
/// `MonographType::Preprint`. Excludes types with more specific semantics
/// (`Book`, `Thesis`, `Report`, ...), where a stray `eprint` field doesn't
/// override the entry-type-driven mapping.
fn is_document_like_monograph_type(t: &MonographType) -> bool {
    matches!(
        t,
        MonographType::Document | MonographType::Manuscript | MonographType::Webpage
    )
}

/// Build a `WorkRelation::Embedded` pointing at a bare `Collection` carrying
/// `series` as its title, so `collection_title()`/`collection_number()`
/// resolve it. Mirrors `relation_collection_title` in citum-schema-data's
/// CSL-JSON conversion path -- a BibLaTeX series and a CSL `collection-title`
/// are the same concept, so both input formats produce the same shape for it
/// rather than a dedicated flat `series` field.
fn series_relation(series: Option<String>) -> Option<WorkRelation> {
    series.map(|title| {
        WorkRelation::Embedded(Box::new(InputReference::Collection(Box::new(Collection {
            title: Some(Title::Single(title)),
            ..Default::default()
        }))))
    })
}

/// Build a Monograph reference with common fields from biblatex.
///
/// Maps biblatex `edition` and `number` fields into canonical `numbering`,
/// treating `report` entry numbers as `NumberingType::Report`, and handles URL parsing.
fn biblatex_monograph(
    r#type: MonographType,
    entry_type: &str,
    ctx: BibRefContext<'_>,
) -> Monograph {
    let field_str = ctx.field_str;
    let rich_field_str = ctx.rich_field_str;
    let contributors = ctx.contributors;

    let series = field_str("series");

    let mut numbering = Vec::new();
    if let Some(ed) = field_str("edition") {
        numbering.push(Numbering {
            r#type: NumberingType::Edition,
            value: ed,
        });
    }
    if let Some(n) = field_str("number") {
        // A `number` alongside `series` is the volume-in-series number, not a
        // generic document number -- `collection_number()` reads it back via
        // `NumberingType::Volume`. The `report` special case is unaffected.
        let numbering_type = if entry_type == "report" {
            NumberingType::Report
        } else if series.is_some() {
            NumberingType::Volume
        } else {
            NumberingType::Number
        };
        numbering.push(Numbering {
            r#type: numbering_type,
            value: n,
        });
    }

    // No intermediate container-title for a bare `@book`/`@report`/etc., so a
    // `series` wraps in a title-less parent (mirroring the CSL-JSON path's
    // identical "book in a series with no intermediate container-title" case)
    // rather than being attached directly as this reference's own container.
    let container = series.map(|series| {
        WorkRelation::Embedded(Box::new(InputReference::Monograph(Box::new(Monograph {
            container: series_relation(Some(series)),
            ..Default::default()
        }))))
    });

    Monograph {
        id: ctx.id,
        r#type,
        title: ctx.title,
        short_title: None,
        container,
        author: ctx.author,
        editor: ctx.editor,
        translator: ctx.translator,
        contributors,
        eprint: ctx.eprint,
        created: DateValue::new(String::new()),
        issued: ctx.issued,
        publisher: ctx.publisher,
        url: field_str("url").and_then(|u| Url::parse(&u).ok()),
        accessed: field_str("urldate").map(DateValue::new),
        language: ctx.language,
        field_languages: HashMap::new(),
        note: field_str("note").map(RichText::Plain),
        abstract_text: rich_field_str("abstract").map(RichText::Plain),
        isbn: field_str("isbn"),
        doi: field_str("doi"),
        ads_bibcode: field_str("bibcode"),
        version: field_str("version"),
        keywords: split_keywords(field_str("keywords")),
        numbering,
        genre: if entry_type == "periodical" {
            Some("periodical".to_string())
        } else {
            rich_field_str("type")
        },
        // biblatex allows `pages` on `@book` (GB/T 7714 引文页码); see
        // `Monograph::pages` doc comment.
        pages: field_str("pages").map(NumOrStr::Str),
        ..Default::default()
    }
}

/// Convert biblatex persons (authors/editors/translators) to a `Contributor`.
///
/// Each person maps to a `StructuredName`, or to a `SimpleName` literal when
/// there is no given name, prefix, or suffix to structure (see
/// [`contributor_from_person`]). A single contributor is returned bare rather
/// than wrapped, matching the shape `push_legacy_contributor` produces for
/// CSL-JSON/RIS input; two or more are wrapped in a `ContributorList`.
pub fn contributors_from_biblatex_persons(persons: &[biblatex_crate::Person]) -> Contributor {
    let mut contributors: Vec<Contributor> = persons.iter().map(contributor_from_person).collect();
    // Match the shape the CSL-JSON/RIS paths produce (`push_legacy_contributor`,
    // crates/citum-schema-data/src/reference/conversion/mod.rs): a single
    // contributor serializes bare, not as a one-element `ContributorList`.
    if contributors.len() == 1 {
        return contributors.remove(0);
    }
    Contributor::ContributorList(ContributorList(contributors))
}

/// Convert a single biblatex `Person` to a `Contributor`.
///
/// Biblatex parses a comma-less name (e.g. `{Plato}`) as family-only — the same
/// shape institutional authors arrive in. When there is no given name, prefix,
/// or suffix to structure, the name is emitted as a `SimpleName` literal rather
/// than a `StructuredName` with an empty `given`, so it round-trips as an
/// authorable mononym instead of a malformed structured name.
fn contributor_from_person(p: &biblatex_crate::Person) -> Contributor {
    if p.given_name.is_empty() && p.prefix.is_empty() && p.suffix.is_empty() {
        return Contributor::SimpleName(SimpleName {
            name: p.name.clone().into(),
            location: None,
            short_name: None,
        });
    }
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
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse_single_entry(source: &str) -> biblatex_crate::Entry {
        let bibliography =
            biblatex_crate::Bibliography::parse(source).expect("biblatex should parse");
        bibliography
            .into_iter()
            .next()
            .expect("bibliography should contain one entry")
    }

    #[test]
    fn biblatex_report_number_maps_to_report_numbering() {
        let entry = parse_single_entry(
            "@report{r1,\n  title = {Report},\n  date = {2024},\n  number = {TR-7}\n}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "report");
        assert_eq!(converted.number(), None);
        assert_eq!(converted.report_number(), Some("TR-7".to_string()));
    }

    #[test]
    fn biblatex_book_number_maps_to_generic_numbering() {
        let entry =
            parse_single_entry("@book{b1,\n  title = {Book},\n  date = {2024},\n  number = {2}\n}");

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.number(), Some("2".to_string()));
        assert_eq!(converted.report_number(), None);
    }

    #[test]
    fn book_series_becomes_collection_title() {
        let entry = parse_single_entry(
            "@book{k1, title = {T}, date = {2024}, series = {Studies in Examples}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            converted.collection_title(),
            Some(Title::Single("Studies in Examples".to_string()))
        );
    }

    #[test]
    fn book_series_and_number_becomes_volume_in_series() {
        let entry = parse_single_entry(
            "@book{k1, title = {T}, date = {2024}, series = {Studies in Examples}, \
             number = {12}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.collection_number(), Some("12".to_string()));
        assert_eq!(converted.number(), None);
    }

    #[test]
    fn book_number_without_series_stays_generic() {
        let entry = parse_single_entry("@book{k1, title = {T}, date = {2024}, number = {12}}");

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.number(), Some("12".to_string()));
        assert_eq!(converted.collection_number(), None);
    }

    #[test]
    fn incollection_series_is_on_parent_collection() {
        let entry = parse_single_entry(
            "@incollection{k1, title = {Chapter}, booktitle = {Book}, date = {2024}, \
             series = {Studies in Examples}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            converted.collection_title(),
            Some(Title::Single("Studies in Examples".to_string()))
        );
    }

    #[rstest]
    #[case::collection("@collection{k1, title = {Collected Work}, series = {Studies in Examples}}")]
    #[case::proceedings(
        "@proceedings{k1, title = {Conference Proceedings}, series = {Studies in Examples}}"
    )]
    fn standalone_collection_series_is_accessible(#[case] source: &str) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            converted.collection_title(),
            Some(Title::Single("Studies in Examples".to_string()))
        );
    }

    #[test]
    fn given_techreport_with_number_when_converted_then_maps_to_report_type_and_report_numbering() {
        let entry = parse_single_entry(
            "@techreport{k1,\n  title = {T},\n  date = {2024},\n  number = {TR-9}\n}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "report");
        assert_eq!(converted.report_number(), Some("TR-9".to_string()));
        assert_eq!(converted.number(), None);
    }

    #[rstest]
    #[case::phdthesis_maps_to_thesis("@phdthesis{k1, title={T}, date={2024}}", "thesis")]
    #[case::mastersthesis_maps_to_thesis("@mastersthesis{k1, title={T}, date={2024}}", "thesis")]
    #[case::thesis_maps_to_thesis("@thesis{k1, title={T}, date={2024}}", "thesis")]
    #[case::online_maps_to_webpage("@online{k1, title={T}, date={2024}}", "webpage")]
    #[case::unpublished_maps_to_manuscript(
        "@unpublished{k1, title={T}, date={2024}}",
        "manuscript"
    )]
    fn given_biblatex_entry_type_when_converted_then_maps_to_expected_monograph_type(
        #[case] source: &str,
        #[case] expected_ref_type: &str,
    ) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), expected_ref_type);
    }

    #[test]
    fn given_translator_field_when_converted_then_translator_is_mapped() {
        let entry = parse_single_entry(
            "@book{b2, title = {Book}, date = {2024}, translator = {Doe, Jane}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let monograph = converted.as_monograph().expect("expected a Monograph");
        assert_eq!(
            monograph.translator,
            Some(Contributor::StructuredName(StructuredName {
                given: "Jane".into(),
                family: "Doe".into(),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            }))
        );
    }

    #[test]
    fn given_thesis_with_institution_and_no_publisher_when_converted_then_institution_becomes_publisher_with_location()
     {
        let entry = parse_single_entry(
            "@phdthesis{t1, title = {T}, date = {2024}, institution = {Wuhan University}, location = {Wuhan}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let monograph = converted.as_monograph().expect("expected a Monograph");
        assert_eq!(
            monograph.publisher,
            Some(Publisher {
                name: "Wuhan University".into(),
                place: Some("Wuhan".into()),
            })
        );
    }

    #[test]
    fn given_title_and_subtitle_when_converted_then_title_is_structured() {
        let entry = parse_single_entry(
            "@book{b3, title = {Main Title}, subtitle = {A Subtitle}, date = {2024}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let monograph = converted.as_monograph().expect("expected a Monograph");
        assert_eq!(
            monograph.title,
            Some(Title::Structured(StructuredTitle {
                full: None,
                main: "Main Title".to_string(),
                sub: Subtitle::String("A Subtitle".to_string()),
            }))
        );
    }

    #[test]
    fn math_chunk_becomes_djot_inline_math() {
        // `Chunk::Math` used to be discarded to an empty string; the
        // `biblatex` crate strips the delimiting `$...$` when parsing, so
        // this must put them back rather than silently dropping the content.
        let entry = parse_single_entry("@book{k1, title = {Energy: $E=mc^2$}, date = {2024}}");

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            converted.title(),
            Some(Title::Single("Energy: $E=mc^2$".to_string()))
        );
    }

    #[test]
    fn multiple_locations_are_joined_with_semicolon() {
        // BibLaTeX's `location`/`organization`/`publisher` are `LiteralList`
        // fields: a bare `and` inside one field value names two places, not
        // the literal text "Boston and London".
        let entry = parse_single_entry(
            "@book{k1, title = {T}, date = {2024}, publisher = {Acme}, \
             location = {Boston and London}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let monograph = converted.as_monograph().expect("expected a Monograph");
        assert_eq!(
            monograph.publisher,
            Some(Publisher {
                name: "Acme".into(),
                place: Some("Boston; London".into()),
            })
        );
    }

    #[test]
    fn multiple_organizations_are_joined_with_semicolon() {
        let entry = parse_single_entry(
            "@standard{s1, title = {T}, date = {2024}, number = {ISO 8601}, \
             organization = {ISO and IEC}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let standard = converted.as_standard().expect("expected a Standard");
        assert_eq!(standard.authority, Some("ISO; IEC".to_string()));
    }

    #[test]
    fn given_incollection_with_isbn_when_converted_then_isbn_is_on_the_parent_collection() {
        let entry = parse_single_entry(
            "@incollection{c1, title = {Chapter}, booktitle = {Book}, date = {2024}, isbn = {978-0-13-468599-1}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let component = converted
            .as_collection_component()
            .expect("expected a CollectionComponent");
        let parent = match component.container.as_ref().expect("expected a container") {
            WorkRelation::Embedded(inner) => inner
                .as_collection()
                .expect("expected an embedded Collection"),
            WorkRelation::Id(_) => panic!("expected an embedded container, not an id reference"),
        };
        assert_eq!(parent.isbn, Some("978-0-13-468599-1".to_string()));
    }

    #[test]
    fn inproceedings_event_fields_are_on_parent_collection() {
        let entry = parse_single_entry(
            "@inproceedings{p1, title = {Paper}, booktitle = {Proceedings}, date = {2024}, \
             eventtitle = {Symposium on Examples}, venue = {Springfield}, \
             eventdate = {2023-06-01}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let component = converted
            .as_collection_component()
            .expect("expected a CollectionComponent");
        let parent = match component.container.as_ref().expect("expected a container") {
            WorkRelation::Embedded(inner) => inner
                .as_collection()
                .expect("expected an embedded Collection"),
            WorkRelation::Id(_) => panic!("expected an embedded container, not an id reference"),
        };
        assert_eq!(parent.r#type, CollectionType::Proceedings);
        let event = match parent.event.as_ref().expect("expected an event") {
            WorkRelation::Embedded(inner) => inner.as_event().expect("expected an Event"),
            WorkRelation::Id(_) => panic!("expected an embedded event, not an id reference"),
        };
        assert_eq!(
            event.title,
            Some(Title::Single("Symposium on Examples".to_string()))
        );
        assert_eq!(event.location, Some("Springfield".to_string()));
        assert_eq!(event.date, Some(DateValue::new("2023-06-01".to_string())));
    }

    #[test]
    fn incollection_eventtitle_is_not_read() {
        // `eventtitle`/`venue`/`eventdate` are only meaningful for the
        // conference itself; an `@incollection` (not a proceedings paper)
        // must not pick them up even if present in the source.
        let entry = parse_single_entry(
            "@incollection{c1, title = {Chapter}, booktitle = {Book}, date = {2024}, \
             eventtitle = {Not A Conference}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let component = converted
            .as_collection_component()
            .expect("expected a CollectionComponent");
        let parent = match component.container.as_ref().expect("expected a container") {
            WorkRelation::Embedded(inner) => inner
                .as_collection()
                .expect("expected an embedded Collection"),
            WorkRelation::Id(_) => panic!("expected an embedded container, not an id reference"),
        };
        assert_eq!(parent.r#type, CollectionType::EditedBook);
        assert!(parent.event.is_none());
    }

    #[test]
    fn incollection_chapter_becomes_numbering() {
        let entry = parse_single_entry(
            "@incollection{c1, title = {Chapter}, booktitle = {Book}, date = {2024}, \
             chapter = {7}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let component = converted
            .as_collection_component()
            .expect("expected a CollectionComponent");
        assert_eq!(
            component.numbering,
            vec![Numbering {
                r#type: NumberingType::Chapter,
                value: "7".to_string(),
            }]
        );
    }

    #[test]
    fn given_biblatex_title_with_escaped_html_nocase_span_when_converted_then_becomes_djot_case_protection()
     {
        // Zotero's builtin BibTeX/BibLaTeX exporter escapes citeproc-js's
        // HTML rich-text convention as `{\textless}span
        // class="nocase"{\textgreater}...`; the `biblatex` parser unescapes
        // it back to literal `<span ...>` before Citum sees it. Confirms the
        // biblatex path (bean csl26-6eoi) converts to Djot on ingestion,
        // matching the CSL-JSON path, rather than leaking verbatim into
        // rendered output.
        let entry = parse_single_entry(
            r#"@book{b4, title = {The genome of {\textless}span class="nocase"{\textgreater}Eucalyptus grandis{\textless}/span{\textgreater}}, date = {2014}}"#,
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            converted.title(),
            Some(Title::Single(
                "The genome of [Eucalyptus grandis]{.nocase}".to_string()
            ))
        );
    }

    #[test]
    fn given_biblatex_booktitle_with_escaped_html_nocase_spans_when_converted_then_parent_collection_title_becomes_djot_case_protection()
     {
        // The gb7714-bench regression (entry gbt7714.8.6.1:5, bean
        // csl26-6eoi): `booktitle` carries the same escaped citeproc-js
        // convention as `title`, but through `build_inbook_reference`'s
        // parent-collection path, which `build_title` never sees.
        let entry = parse_single_entry(
            r#"@inproceedings{c1, title = {Advances in holographic photoelasticity}, booktitle = {{\textless}span class="nocase"{\textgreater}Symposium on Applications of Holography in Mechanics{\textless}/span{\textgreater}}, date = {1971}}"#,
        );

        let converted = input_reference_from_biblatex(&entry);

        let component = converted
            .as_collection_component()
            .expect("expected a CollectionComponent");
        let parent = match component.container.as_ref().expect("expected a container") {
            WorkRelation::Embedded(inner) => inner
                .as_collection()
                .expect("expected an embedded Collection"),
            WorkRelation::Id(_) => panic!("expected an embedded container, not an id reference"),
        };
        assert_eq!(
            parent.title,
            Some(Title::Single(
                "[Symposium on Applications of Holography in Mechanics]{.nocase}".to_string()
            ))
        );
    }

    #[rstest]
    #[case::proceedings_maps_to_collection(
        "@proceedings{p1, title={T}, date={2024}}",
        "collection"
    )]
    #[case::mvproceedings_maps_to_collection(
        "@mvproceedings{p1, title={T}, date={2024}}",
        "collection"
    )]
    #[case::collection_maps_to_book("@collection{p1, title={T}, date={2024}}", "book")]
    #[case::mvcollection_maps_to_book("@mvcollection{p1, title={T}, date={2024}}", "book")]
    fn collection_and_proceedings_entry_types_map_to_collection_class(
        #[case] source: &str,
        #[case] expected_ref_type: &str,
    ) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), expected_ref_type);
        assert!(converted.as_collection().is_some());
    }

    #[test]
    fn standalone_proceedings_sets_event_and_editor() {
        let entry = parse_single_entry(
            "@proceedings{p1, title = {Proceedings of Examples}, editor = {Doe, John}, \
             date = {2024}, eventtitle = {Symposium on Examples}, venue = {Springfield}, \
             eventdate = {2023-06-01}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let collection = converted.as_collection().expect("expected a Collection");
        assert_eq!(collection.r#type, CollectionType::Proceedings);
        let event = match collection.event.as_ref().expect("expected an event") {
            WorkRelation::Embedded(inner) => inner.as_event().expect("expected an Event"),
            WorkRelation::Id(_) => panic!("expected an embedded event, not an id reference"),
        };
        assert_eq!(
            event.title,
            Some(Title::Single("Symposium on Examples".to_string()))
        );
        assert_eq!(event.location, Some("Springfield".to_string()));
        assert_eq!(
            converted.editor(),
            Some(Contributor::StructuredName(StructuredName {
                given: "John".into(),
                family: "Doe".into(),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            }))
        );
    }

    #[test]
    fn patent_with_number_and_holder_maps_to_patent() {
        // Double braces protect the corporate name from BibTeX's
        // comma-less "Given Family" name-splitting convention, matching how
        // real .bib files mark up an institutional holder.
        let entry = parse_single_entry(
            "@patent{p1, title = {Widget}, date = {2024}, number = {US7,347,809}, \
             holder = {{Acme Corp}}, location = {US}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "patent");
        let patent = converted.as_patent().expect("expected a Patent");
        assert_eq!(patent.patent_number, "US7,347,809");
        assert_eq!(
            patent.assignee,
            Some(Contributor::SimpleName(SimpleName {
                name: "Acme Corp".into(),
                location: None,
                short_name: None,
            }))
        );
        assert_eq!(patent.jurisdiction, Some("US".to_string()));
    }

    #[rstest]
    #[case::missing("@patent{p1, title = {Widget}, date = {2024}}")]
    #[case::empty("@patent{p1, title = {Widget}, date = {2024}, number = {}}")]
    #[case::whitespace("@patent{p1, title = {Widget}, date = {2024}, number = {   }}")]
    fn patent_without_nonblank_number_stays_on_fallback(#[case] source: &str) {
        // `Patent.patent_number` is required (non-`Option`); a `@patent`
        // with no nonblank `number` must not be given an empty identifier.
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "document");
        assert!(converted.as_patent().is_none());
    }

    #[test]
    fn dataset_maps_to_dataset() {
        let entry = parse_single_entry(
            "@dataset{d1, title = {Survey Data}, date = {2024}, version = {1.2}, \
             publisher = {Zenodo}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "dataset");
        let dataset = converted.as_dataset().expect("expected a Dataset");
        assert_eq!(dataset.version, Some("1.2".to_string()));
        assert_eq!(
            dataset.publisher,
            Some(Publisher {
                name: "Zenodo".into(),
                place: None,
            })
        );
    }

    #[test]
    fn software_maps_to_software() {
        let entry = parse_single_entry(
            "@software{s1, title = {Widget Toolkit}, date = {2024}, version = {4.1.0}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "software");
        let software = converted.as_software().expect("expected Software");
        assert_eq!(software.version, Some("4.1.0".to_string()));
    }

    #[test]
    fn standard_with_number_maps_to_standard() {
        let entry = parse_single_entry(
            "@standard{s1, title = {Date and Time Format}, date = {2024}, number = {ISO 8601}, \
             organization = {ISO}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "standard");
        let standard = converted.as_standard().expect("expected a Standard");
        assert_eq!(standard.standard_number, "ISO 8601");
        assert_eq!(standard.authority, Some("ISO".to_string()));
    }

    #[rstest]
    #[case::missing("@standard{s1, title = {Untitled Standard}, date = {2024}}")]
    #[case::empty("@standard{s1, title = {Untitled Standard}, date = {2024}, number = {}}")]
    #[case::whitespace("@standard{s1, title = {Untitled Standard}, date = {2024}, number = {   }}")]
    fn standard_without_nonblank_number_stays_on_fallback(#[case] source: &str) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "document");
        assert!(converted.as_standard().is_none());
    }

    #[test]
    fn periodical_uses_document_compatibility_contract() {
        // The current model cannot represent issue → journal hierarchy, so
        // preserve the established document contract and canonical genre.
        let entry = parse_single_entry(
            "@periodical{p1, title = {Journal of Examples}, date = {2024}, issn = {1234-5678}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "periodical");
        assert_eq!(converted.issued(), Some(DateValue::new("2024".to_string())));
        let monograph = converted.as_monograph().expect("expected a Monograph");
        assert_eq!(
            monograph.title,
            Some(Title::Single("Journal of Examples".to_string()))
        );
        assert_eq!(monograph.genre.as_deref(), Some("periodical"));
    }

    #[rstest]
    #[case::reference_maps_to_book("@reference{r1, title={T}, date={2024}}", "book")]
    #[case::mvreference_maps_to_book("@mvreference{r1, title={T}, date={2024}}", "book")]
    fn reference_entry_types_map_to_book(#[case] source: &str, #[case] expected_ref_type: &str) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), expected_ref_type);
    }

    #[test]
    fn inreference_maps_like_incollection() {
        let entry = parse_single_entry(
            "@inreference{i1, title = {Entry}, booktitle = {Encyclopedia}, date = {2024}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        let component = converted
            .as_collection_component()
            .expect("expected a CollectionComponent");
        let parent = match component.container.as_ref().expect("expected a container") {
            WorkRelation::Embedded(inner) => inner
                .as_collection()
                .expect("expected an embedded Collection"),
            WorkRelation::Id(_) => panic!("expected an embedded container, not an id reference"),
        };
        assert_eq!(parent.r#type, CollectionType::EditedBook);
        assert_eq!(
            parent.title,
            Some(Title::Single("Encyclopedia".to_string()))
        );
        assert_eq!(converted.ref_type(), "entry");
    }

    #[test]
    fn inproceedings_maps_to_paper_conference() {
        let entry = parse_single_entry(
            "@inproceedings{p1, title = {Paper}, booktitle = {Proceedings}, date = {2024}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "paper-conference");
        assert_eq!(
            converted
                .as_collection_component()
                .expect("expected a CollectionComponent")
                .r#type,
            MonographComponentType::Document
        );
    }

    #[test]
    fn article_with_no_journal_and_an_eprint_becomes_a_preprint() {
        // Mirrors `CSL_TYPE_MAP`'s rule that a container-less CSL-JSON
        // `article` is a standalone preprint, not a truncated journal article.
        let entry = parse_single_entry(
            "@article{k1, title = {T}, date = {2024}, eprint = {2301.00001}, \
             eprinttype = {arXiv}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "preprint");
        let monograph = converted.as_monograph().expect("expected a Monograph");
        assert_eq!(
            monograph.eprint,
            Some(EprintInfo {
                id: "2301.00001".to_string(),
                server: "arxiv".to_string(),
                class: None,
            })
        );
    }

    #[test]
    fn article_with_a_journal_and_an_eprint_keeps_eprint_metadata() {
        let entry = parse_single_entry(
            "@article{k1, title = {T}, date = {2024}, journaltitle = {J}, \
             eprint = {2301.00001}, eprinttype = {arXiv}, eprintclass = {cs.DL}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "article-journal");
        let component = converted
            .as_serial_component()
            .expect("expected a SerialComponent");
        assert_eq!(
            component.eprint,
            Some(EprintInfo {
                id: "2301.00001".to_string(),
                server: "arxiv".to_string(),
                class: Some("cs.DL".to_string()),
            })
        );
    }

    #[rstest]
    #[case::misc_becomes_preprint(
        "@misc{k1, title = {T}, date = {2024}, eprint = {2301.00001}}",
        "preprint"
    )]
    #[case::unpublished_becomes_preprint(
        "@unpublished{k1, title = {T}, date = {2024}, eprint = {2301.00001}}",
        "preprint"
    )]
    #[case::online_becomes_preprint(
        "@online{k1, title = {T}, date = {2024}, eprint = {2301.00001}}",
        "preprint"
    )]
    #[case::book_keeps_its_type(
        "@book{k1, title = {T}, date = {2024}, eprint = {2301.00001}}",
        "book"
    )]
    fn eprint_field_type_flip_follows_precedence_rule(
        #[case] source: &str,
        #[case] expected_ref_type: &str,
    ) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), expected_ref_type);
    }

    #[test]
    fn bare_eprint_keeps_identifier_and_uses_empty_server() {
        let entry =
            parse_single_entry("@misc{k1, title = {T}, date = {2024}, eprint = {2301.00001}}");

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "preprint");
        assert_eq!(converted.eprint_id().as_deref(), Some("2301.00001"));
        assert_eq!(converted.eprint_server().as_deref(), Some(""));
    }

    #[rstest]
    #[case::empty("@misc{k1, title = {T}, date = {2024}, eprint = {}}")]
    #[case::whitespace("@misc{k1, title = {T}, date = {2024}, eprint = {   }}")]
    fn blank_eprint_does_not_promote_to_preprint(#[case] source: &str) {
        let entry = parse_single_entry(source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(converted.ref_type(), "document");
        assert_eq!(converted.eprint_id(), None);
    }

    /// Serialize `reference` to YAML the same way `citum convert refs` does.
    fn to_yaml(reference: &InputReference) -> String {
        serde_yaml::to_string(reference).expect("reference should serialize")
    }

    #[test]
    fn given_article_with_mononym_author_when_serialized_then_author_is_a_simple_name() {
        // Regression test for bean csl26-7ab8: the biblatex converter builds
        // `InputReference`s directly rather than through deserialization, so the
        // legacy `author` shorthand it filled was never folded into the
        // canonical `contributors` vec and vanished on write. Asserting on
        // `.author()` (which falls back to the shorthand) would not have caught
        // this — the assertion must cross the serialization boundary.
        let entry = parse_single_entry("@article{key, author = {Author}, title = {Title} }");

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: serial-component\n\
             id: key\n\
             type: article\n\
             title: Title\n\
             contributors:\n\
             - roles:\n  \
             - author\n  \
             contributor:\n    \
             name: Author\n\
             container:\n  \
             class: serial\n  \
             type: academic-journal\n"
        );
    }

    #[test]
    fn given_article_with_structured_author_when_serialized_then_single_contributor_is_unwrapped() {
        // A single contributor must serialize bare (`contributor: {..}`), matching
        // the shape `push_legacy_contributor` produces for CSL-JSON/RIS input —
        // not as a one-element `ContributorList` sequence.
        let entry = parse_single_entry("@article{key, author = {Smith, Jane}, title = {Title} }");

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: serial-component\n\
             id: key\n\
             type: article\n\
             title: Title\n\
             contributors:\n\
             - roles:\n  \
             - author\n  \
             contributor:\n    \
             given: Jane\n    \
             family: Smith\n\
             container:\n  \
             class: serial\n  \
             type: academic-journal\n"
        );
    }

    #[test]
    fn given_article_with_editor_when_serialized_then_editor_is_on_the_parent_serial() {
        let entry = parse_single_entry(
            "@article{key, author = {Smith, Jane}, editor = {Doe, John}, title = {Title} }",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: serial-component\n\
             id: key\n\
             type: article\n\
             title: Title\n\
             contributors:\n\
             - roles:\n  \
             - author\n  \
             contributor:\n    \
             given: Jane\n    \
             family: Smith\n\
             container:\n  \
             class: serial\n  \
             type: academic-journal\n  \
             contributors:\n  \
             - roles:\n    \
             - editor\n    \
             contributor:\n      \
             given: John\n      \
             family: Doe\n"
        );
    }

    #[test]
    fn given_incollection_with_translator_when_serialized_then_translator_is_on_the_component_only()
    {
        let entry = parse_single_entry(
            "@incollection{key, author = {Smith, Jane}, translator = {Doe, John}, \
             title = {Title}, booktitle = {Book} }",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: collection-component\n\
             id: key\n\
             type: chapter\n\
             title: Title\n\
             contributors:\n\
             - roles:\n  \
             - author\n  \
             contributor:\n    \
             given: Jane\n    \
             family: Smith\n\
             - roles:\n  \
             - translator\n  \
             contributor:\n    \
             given: John\n    \
             family: Doe\n\
             container:\n  \
             class: collection\n  \
             type: edited-book\n  \
             title: Book\n"
        );
    }

    #[test]
    fn given_book_with_pages_and_translator_when_serialized_then_both_are_present() {
        let entry = parse_single_entry(
            "@book{key, author = {Smith, Jane}, translator = {Doe, John}, title = {Title}, \
             pages = {1-20} }",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: monograph\n\
             id: key\n\
             type: book\n\
             title: Title\n\
             contributors:\n\
             - roles:\n  \
             - author\n  \
             contributor:\n    \
             given: Jane\n    \
             family: Smith\n\
             - roles:\n  \
             - translator\n  \
             contributor:\n    \
             given: John\n    \
             family: Doe\n\
             pages: 1-20\n"
        );
    }

    #[test]
    fn given_converted_reference_when_round_tripped_through_yaml_then_it_is_unchanged() {
        // Proves the emitted YAML is readable by the deserializer and that
        // `normalize_contributors` is idempotent (re-normalizing an already
        // normalized reference must not change it).
        let entry = parse_single_entry(
            "@incollection{key, author = {Smith, Jane}, editor = {Doe, John}, \
             translator = {Lee, Kim}, title = {Title}, booktitle = {Book} }",
        );
        let converted = input_reference_from_biblatex(&entry);

        let yaml = to_yaml(&converted);
        let round_tripped: InputReference =
            serde_yaml::from_str(&yaml).expect("emitted YAML should deserialize");

        assert_eq!(converted, round_tripped);
    }

    #[rstest]
    #[case::annotator("annotator", "annotator")]
    #[case::commentator("commentator", "commentator")]
    #[case::foreword("foreword", "foreword-author")]
    #[case::introduction("introduction", "introduction-author")]
    #[case::afterword("afterword", "afterword-author")]
    fn editorial_sub_role_field_maps_to_typed_contributor_role(
        #[case] biblatex_field: &str,
        #[case] expected_role: &str,
    ) {
        let source =
            format!("@book{{k1, title = {{T}}, date = {{2024}}, {biblatex_field} = {{Roe, Sam}}}}");
        let entry = parse_single_entry(&source);

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            format!(
                "class: monograph\n\
                 id: k1\n\
                 type: book\n\
                 title: T\n\
                 contributors:\n\
                 - roles:\n  \
                 - {expected_role}\n  \
                 contributor:\n    \
                 given: Sam\n    \
                 family: Roe\n\
                 issued: '2024'\n"
            )
        );
    }

    #[test]
    fn editor_and_editora_with_different_editor_types_stay_distinct() {
        // Regression: `entry.editors()` returns one group per editor* field
        // plus its `EditorType`; extraction used to flatten all of it into a
        // single undifferentiated `editor` field, discarding the type. Only
        // the `EditorType::Editor` group should become the `editor`
        // shorthand -- other groups (here, `editora`'s `compiler` type) go
        // onto `contributors` directly.
        let entry = parse_single_entry(
            "@book{k1, title = {T}, date = {2024}, editor = {Doe, John}, \
             editora = {Roe, Sam}, editoratype = {compiler}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: monograph\n\
             id: k1\n\
             type: book\n\
             title: T\n\
             contributors:\n\
             - roles:\n  \
             - compiler\n  \
             contributor:\n    \
             given: Sam\n    \
             family: Roe\n\
             - roles:\n  \
             - editor\n  \
             contributor:\n    \
             given: John\n    \
             family: Doe\n\
             issued: '2024'\n"
        );
    }

    #[test]
    fn editora_with_untyped_editor_type_degrades_to_unknown_role() {
        let entry = parse_single_entry(
            "@book{k1, title = {T}, date = {2024}, editora = {Roe, Sam}, editoratype = {organizer}}",
        );

        let converted = input_reference_from_biblatex(&entry);

        assert_eq!(
            to_yaml(&converted),
            "class: monograph\n\
             id: k1\n\
             type: book\n\
             title: T\n\
             contributors:\n\
             - roles:\n  \
             - organizer\n  \
             contributor:\n    \
             given: Sam\n    \
             family: Roe\n\
             issued: '2024'\n"
        );
    }
}
