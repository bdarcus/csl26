/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Hierarchical work types: monographs, collections, serials, and their components.

use super::common::{
    ArchiveInfo, EprintInfo, FieldLanguageMap, HasNumbering, LangID, NormalizeNumbering, NumOrStr,
    Numbering, Publisher, RefID, RichText, Title,
};
use crate::reference::WorkRelation;
use crate::reference::contributor::{
    Contributor, ContributorEntry, ContributorList, ContributorRole,
};
use crate::reference::date::DateValue;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "bindings")]
use specta::Type;
use std::collections::HashMap;
use url::Url;

/// Fold named contributor shorthands into the contributors vec.
///
/// Explicit `contributors` entries come first (source order). Named
/// shorthands are appended only when no existing entry already carries
/// the same role and contributor value.
fn fold_contributors(
    mut contributors: Vec<ContributorEntry>,
    shorthands: &[(ContributorRole, Option<&Contributor>)],
) -> Vec<ContributorEntry> {
    for (role, maybe_c) in shorthands {
        if let Some(c) = maybe_c
            && !contributors
                .iter()
                .any(|entry| entry.roles.contains(role) && &entry.contributor == *c)
        {
            contributors.push(ContributorEntry {
                roles: role.clone().into(),
                contributor: (*c).clone(),
                gender: None,
            });
        }
    }
    contributors
}

/// Collect contributors with a given role from a slice of entries.
fn collect_contributors_by_role(
    entries: &[ContributorEntry],
    role: &ContributorRole,
) -> Option<Contributor> {
    let matching: Vec<&Contributor> = entries
        .iter()
        .filter(|entry| entry.roles.contains(role))
        .map(|entry| &entry.contributor)
        .collect();

    match matching.as_slice() {
        [] => None,
        [single] => Some((*single).clone()),
        _ => Some(Contributor::ContributorList(ContributorList(
            matching.into_iter().cloned().collect(),
        ))),
    }
}

/// Per-role contributor views derived from the unified contributors vec.
struct ContributorViews {
    author: Option<Contributor>,
    editor: Option<Contributor>,
    translator: Option<Contributor>,
}

/// Fold shorthand fields into the contributors vec and re-derive per-role views.
///
/// Pass `None` for roles the calling type does not have (e.g. Collection has
/// no author shorthand). Each view is derived from the folded contributors list
/// and will be `None` when that role has no entries.
fn reconcile_contributors(
    contributors: Vec<ContributorEntry>,
    author: Option<Contributor>,
    editor: Option<Contributor>,
    translator: Option<Contributor>,
) -> (Vec<ContributorEntry>, ContributorViews) {
    let folded = fold_contributors(
        contributors,
        &[
            (ContributorRole::Author, author.as_ref()),
            (ContributorRole::Editor, editor.as_ref()),
            (ContributorRole::Translator, translator.as_ref()),
        ],
    );
    let views = ContributorViews {
        author: collect_contributors_by_role(&folded, &ContributorRole::Author),
        editor: collect_contributors_by_role(&folded, &ContributorRole::Editor),
        translator: collect_contributors_by_role(&folded, &ContributorRole::Translator),
    };
    (folded, views)
}

/// A monograph, such as a book or a report, is a monolithic work published or produced as a complete entity.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(from = "MonographDeser", rename_all = "kebab-case")]
pub struct Monograph {
    /// Unique identifier for this reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RefID>,
    /// Subtype for style-directed formatting.
    pub r#type: MonographType,
    /// Title of the monographic work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// Optional short form of the title for style-directed rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_title: Option<String>,
    /// Title of an individual volume within the monographic work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_title: Option<String>,
    /// The primary container for this work (e.g., a multivolume set or series).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<WorkRelation>,
    /// Author(s) of the work.
    #[serde(skip_serializing)]
    pub author: Option<Contributor>,
    /// Editor(s) of the work.
    #[serde(skip_serializing)]
    pub editor: Option<Contributor>,
    /// Translator(s) of the work.
    #[serde(skip_serializing)]
    pub translator: Option<Contributor>,
    /// Unified contributor list with explicit role tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<ContributorEntry>,
    /// Creation or origination date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub created: DateValue,
    /// Publication date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub issued: DateValue,
    /// Publisher of the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,
    /// URL for the work.
    #[serde(alias = "URL", skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    /// Date the URL was accessed.
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<DateValue>,
    /// BCP 47 language of the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LangID>,
    /// Per-field language overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_languages: FieldLanguageMap,
    /// Freeform note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<RichText>,
    /// Abstract or summary of the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<RichText>,
    /// ISBN identifier.
    #[serde(alias = "ISBN", skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    /// DOI identifier.
    #[serde(alias = "DOI", skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// ADS bibcode identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ads_bibcode: Option<String>,
    /// Version or revision identifier for the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Volume number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Edition (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Generic document number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// Part number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    /// Supplement number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplement_number: Option<String>,
    /// Printing number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printing_number: Option<String>,
    /// Numbering identifiers (e.g., volume, issue, edition).
    /// Flat shorthand fields are accepted on input for authoring ergonomics and normalized
    /// into canonical `numbering` entries during deserialization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbering: Vec<Numbering>,
    /// Free-text genre descriptor using kebab-case canonical forms (e.g., `"phd-thesis"`, `"short-film"`).
    /// See `docs/reference/GENRE_AND_MEDIUM_VALUES.md` for canonical values and `docs/policies/ENUM_VOCABULARY_POLICY.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Free-text medium descriptor using kebab-case canonical forms (e.g., `"film"`, `"television"`).
    /// See `docs/reference/GENRE_AND_MEDIUM_VALUES.md` for canonical values and `docs/policies/ENUM_VOCABULARY_POLICY.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    /// Archive or repository name for unpublished material.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
    /// Archive location, shelfmark, or call number for unpublished material.
    #[serde(alias = "archive_location", skip_serializing_if = "Option::is_none")]
    pub archive_location: Option<String>,
    /// Structured archival location metadata. When present, preferred over legacy `archive` and `archive_location`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_info: Option<ArchiveInfo>,
    /// Preprint server identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eprint: Option<EprintInfo>,
    /// Keywords or subject tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// Original publication relation (for reprints or translations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<WorkRelation>,
    /// Event or performance associated with the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<WorkRelation>,
    /// Publication status (e.g., `"forthcoming"`, `"in press"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Date the work became or will become publicly available.
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_date: Option<DateValue>,
    /// Copyright year, a publication-year substitute used when the true
    /// issue date is unknown (GB/T 7714 §7.5.4.3's `c1988`).
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<DateValue>,
    /// Printing/impression year, another publication-year substitute
    /// (GB/T 7714 §7.5.4.3's `1995印刷`).
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printing: Option<DateValue>,
    /// Physical dimensions or format (e.g., `"24 x 30 cm"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// Duration or running time in ISO 8601 or freeform (e.g., `"PT2H30M"`, `"90 min"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// Reference list count or citation string (e.g., `"42 refs"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<String>,
    /// Cartographic scale for maps and globes (e.g., `"1:250,000"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<String>,
    /// Cited page or page range rendered as part of the entry (GB/T 7714
    /// 引文页码; biblatex allows `pages` on `@book`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<NumOrStr>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// reference produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Monograph {
    /// Fold the `author`/`editor`/`translator` shorthands into the canonical
    /// `contributors` vec and re-derive the per-role views.
    ///
    /// Deserialized values are already normalized by `From<MonographDeser>`; this
    /// exists for references built directly in Rust (e.g. the biblatex converter),
    /// whose shorthand fields are `skip_serializing` and would otherwise be lost
    /// on write. Idempotent — safe to call on an already-normalized value.
    pub(crate) fn normalize_contributors(&mut self) {
        let (contributors, views) = reconcile_contributors(
            std::mem::take(&mut self.contributors),
            self.author.take(),
            self.editor.take(),
            self.translator.take(),
        );
        self.contributors = contributors;
        self.author = views.author;
        self.editor = views.editor;
        self.translator = views.translator;
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
struct MonographDeser {
    id: Option<RefID>,
    r#type: MonographType,
    title: Option<Title>,
    short_title: Option<String>,
    volume_title: Option<String>,
    container: Option<WorkRelation>,
    author: Option<Contributor>,
    editor: Option<Contributor>,
    translator: Option<Contributor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contributors: Vec<ContributorEntry>,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    created: DateValue,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    issued: DateValue,
    publisher: Option<Publisher>,
    #[serde(alias = "URL")]
    url: Option<Url>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    accessed: Option<DateValue>,
    language: Option<LangID>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    field_languages: FieldLanguageMap,
    note: Option<RichText>,
    abstract_text: Option<RichText>,
    #[serde(alias = "ISBN")]
    isbn: Option<String>,
    #[serde(alias = "DOI")]
    doi: Option<String>,
    ads_bibcode: Option<String>,
    version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    edition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    part_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supplement_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    printing_number: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    numbering: Vec<Numbering>,
    genre: Option<String>,
    medium: Option<String>,
    archive: Option<String>,
    #[serde(alias = "archive_location")]
    archive_location: Option<String>,
    archive_info: Option<ArchiveInfo>,
    eprint: Option<EprintInfo>,
    keywords: Option<Vec<String>>,
    original: Option<WorkRelation>,
    event: Option<WorkRelation>,
    status: Option<String>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    available_date: Option<DateValue>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    copyright: Option<DateValue>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    printing: Option<DateValue>,
    size: Option<String>,
    duration: Option<String>,
    references: Option<String>,
    scale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pages: Option<NumOrStr>,
    #[serde(flatten, default)]
    unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl From<MonographDeser> for Monograph {
    fn from(raw: MonographDeser) -> Self {
        let mut monograph = Self {
            id: raw.id,
            r#type: raw.r#type,
            title: raw.title,
            short_title: raw.short_title,
            volume_title: raw.volume_title,
            container: raw.container,
            author: raw.author,
            editor: raw.editor,
            translator: raw.translator,
            contributors: raw.contributors,
            created: raw.created,
            issued: raw.issued,
            publisher: raw.publisher,
            url: raw.url,
            accessed: raw.accessed,
            language: raw.language,
            field_languages: raw.field_languages,
            note: raw.note,
            abstract_text: raw.abstract_text,
            isbn: raw.isbn,
            doi: raw.doi,
            ads_bibcode: raw.ads_bibcode,
            version: raw.version,
            volume: raw.volume,
            issue: raw.issue,
            edition: raw.edition,
            number: raw.number,
            part_number: raw.part_number,
            supplement_number: raw.supplement_number,
            printing_number: raw.printing_number,
            numbering: raw.numbering,
            genre: raw.genre,
            medium: raw.medium,
            archive: raw.archive,
            archive_location: raw.archive_location,
            archive_info: raw.archive_info,
            eprint: raw.eprint,
            keywords: raw.keywords,
            original: raw.original,
            event: raw.event,
            status: raw.status,
            available_date: raw.available_date,
            copyright: raw.copyright,
            printing: raw.printing,
            size: raw.size,
            duration: raw.duration,
            references: raw.references,
            scale: raw.scale,
            pages: raw.pages,
            unknown_fields: raw.unknown_fields,
        };
        monograph.normalize_contributors();
        monograph.normalize_numbering();
        monograph
    }
}

crate::tolerant_enum! {
    /// Discriminates monograph subtypes for style-directed formatting.
    #[derive(Debug, Default, Clone, PartialEq)]
    pub enum MonographType {
        /// A book or monograph (default).
        #[default]
        Book = "book",
        /// A technical manual or user guide.
        Manual = "manual",
        /// A technical or institutional report.
        Report = "report",
        /// An academic thesis or dissertation.
        Thesis = "thesis",
        /// A webpage or standalone web document.
        Webpage = "webpage",
        /// A standalone post (e.g., social media, forum).
        Post = "post",
        /// An interview treated as a standalone monographic source.
        Interview = "interview",
        /// An unpublished manuscript or archival document.
        Manuscript = "manuscript",
        /// A preprint hosted on a preprint server (arXiv, bioRxiv, SSRN, etc.).
        ///
        /// The preprint server has a custodial relationship with the work (hosting and
        /// preservation), not an editorial one. This parallels an archived manuscript
        /// held by a repository.
        Preprint = "preprint",
        /// A letter, email, or other personal communication.
        PersonalCommunication = "personal-communication",
        /// A generic standalone document that does not fit a more specific subtype.
        Document = "document"
    }
}

/// A collection of works, such as an anthology or proceedings.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(from = "CollectionDeser", rename_all = "kebab-case")]
pub struct Collection {
    /// Unique identifier for this reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RefID>,
    /// Collection subtype for style-directed formatting.
    pub r#type: CollectionType,
    /// Title of the collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// Optional short form of the title for style-directed rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_title: Option<String>,
    /// The primary container for this collection (e.g., a series).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<WorkRelation>,
    /// Editor(s) of the collection.
    #[serde(skip_serializing)]
    pub editor: Option<Contributor>,
    /// Translator(s) of the collection.
    #[serde(skip_serializing)]
    pub translator: Option<Contributor>,
    /// Unified contributor list with explicit role tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<ContributorEntry>,
    /// Creation or origination date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub created: DateValue,
    /// Publication date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub issued: DateValue,
    /// Publisher of the collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,
    /// Volume number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Edition (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Generic document number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// Part number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    /// Supplement number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplement_number: Option<String>,
    /// Printing number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printing_number: Option<String>,
    /// Numbering identifiers (e.g., volume, collection number).
    /// Flat shorthand fields are accepted on input for authoring ergonomics and normalized
    /// into canonical `numbering` entries during deserialization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbering: Vec<Numbering>,
    /// URL for the collection.
    #[serde(alias = "URL", skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    /// Date the URL was accessed.
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<DateValue>,
    /// BCP 47 language of the collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LangID>,
    /// Per-field language overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_languages: FieldLanguageMap,
    /// Freeform note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<RichText>,
    /// ISBN identifier.
    #[serde(alias = "ISBN", skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    /// Originating event (e.g., conference) for proceedings-type containers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<WorkRelation>,
    /// Keywords or subject tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// reference produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Collection {
    /// Fold the `editor`/`translator` shorthands into the canonical `contributors`
    /// vec and re-derive the per-role views. See [`Monograph::normalize_contributors`]
    /// for why this exists. Idempotent.
    pub(crate) fn normalize_contributors(&mut self) {
        let (contributors, views) = reconcile_contributors(
            std::mem::take(&mut self.contributors),
            None,
            self.editor.take(),
            self.translator.take(),
        );
        self.contributors = contributors;
        self.editor = views.editor;
        self.translator = views.translator;
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
struct CollectionDeser {
    id: Option<RefID>,
    r#type: CollectionType,
    title: Option<Title>,
    short_title: Option<String>,
    container: Option<WorkRelation>,
    editor: Option<Contributor>,
    translator: Option<Contributor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contributors: Vec<ContributorEntry>,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    created: DateValue,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    issued: DateValue,
    publisher: Option<Publisher>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    edition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    part_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supplement_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    printing_number: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    numbering: Vec<Numbering>,
    #[serde(alias = "URL")]
    url: Option<Url>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    accessed: Option<DateValue>,
    language: Option<LangID>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    field_languages: FieldLanguageMap,
    note: Option<RichText>,
    #[serde(alias = "ISBN")]
    isbn: Option<String>,
    event: Option<WorkRelation>,
    keywords: Option<Vec<String>>,
    #[serde(flatten, default)]
    unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl From<CollectionDeser> for Collection {
    fn from(raw: CollectionDeser) -> Self {
        let mut collection = Self {
            id: raw.id,
            r#type: raw.r#type,
            title: raw.title,
            short_title: raw.short_title,
            container: raw.container,
            editor: raw.editor,
            translator: raw.translator,
            contributors: raw.contributors,
            created: raw.created,
            issued: raw.issued,
            publisher: raw.publisher,
            volume: raw.volume,
            issue: raw.issue,
            edition: raw.edition,
            number: raw.number,
            part_number: raw.part_number,
            supplement_number: raw.supplement_number,
            printing_number: raw.printing_number,
            numbering: raw.numbering,
            url: raw.url,
            accessed: raw.accessed,
            language: raw.language,
            field_languages: raw.field_languages,
            note: raw.note,
            isbn: raw.isbn,
            event: raw.event,
            keywords: raw.keywords,
            unknown_fields: raw.unknown_fields,
        };
        collection.normalize_contributors();
        collection.normalize_numbering();
        collection
    }
}

crate::tolerant_enum! {
    /// Discriminates collection subtypes for style-directed formatting.
    #[derive(Debug, Default, Clone, PartialEq)]
    pub enum CollectionType {
        /// A curated anthology of independent works (e.g., short stories, essays).
        #[default]
        Anthology = "anthology",
        /// Published proceedings of a conference or symposium.
        Proceedings = "proceedings",
        /// A book assembled from contributions by multiple authors under an editor.
        EditedBook = "edited-book",
        /// An edited volume that may span multiple books or a series.
        EditedVolume = "edited-volume"
    }
}

/// A component of a larger monograph, such as a chapter in a book.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(from = "CollectionComponentDeser", rename_all = "kebab-case")]
pub struct CollectionComponent {
    /// Unique identifier for this reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RefID>,
    /// Component subtype for style-directed formatting.
    pub r#type: MonographComponentType,
    /// Title of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// Author(s) of the component.
    #[serde(skip_serializing)]
    pub author: Option<Contributor>,
    /// Translator(s) of the component.
    #[serde(skip_serializing)]
    pub translator: Option<Contributor>,
    /// Unified contributor list with explicit role tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<ContributorEntry>,
    /// Creation or origination date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub created: DateValue,
    /// Publication date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub issued: DateValue,
    /// The parent collection or monograph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<WorkRelation>,
    /// Volume number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Edition (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Generic document number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// Part number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    /// Supplement number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplement_number: Option<String>,
    /// Printing number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printing_number: Option<String>,
    /// Numbering identifiers (e.g., chapter number, part number).
    /// Flat shorthand fields are accepted on input for authoring ergonomics and normalized
    /// into canonical `numbering` entries during deserialization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbering: Vec<Numbering>,
    /// Page range within the parent container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<NumOrStr>,
    /// URL for the component.
    #[serde(alias = "URL", skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    /// Date the URL was accessed.
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<DateValue>,
    /// BCP 47 language of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LangID>,
    /// Per-field language overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_languages: FieldLanguageMap,
    /// Freeform note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<RichText>,
    /// Abstract or summary of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<RichText>,
    /// DOI identifier.
    #[serde(alias = "DOI", skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Free-text genre descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Free-text medium descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    /// Publication status (e.g., `"forthcoming"`, `"last modified"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Structured archival location metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_info: Option<ArchiveInfo>,
    /// Preprint server identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eprint: Option<EprintInfo>,
    /// Keywords or subject tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// Original publication relation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<WorkRelation>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// reference produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl CollectionComponent {
    /// Fold the `author`/`translator` shorthands into the canonical `contributors`
    /// vec and re-derive the per-role views. See [`Monograph::normalize_contributors`]
    /// for why this exists. Idempotent.
    pub(crate) fn normalize_contributors(&mut self) {
        let (contributors, views) = reconcile_contributors(
            std::mem::take(&mut self.contributors),
            self.author.take(),
            None,
            self.translator.take(),
        );
        self.contributors = contributors;
        self.author = views.author;
        self.translator = views.translator;
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
struct CollectionComponentDeser {
    id: Option<RefID>,
    r#type: MonographComponentType,
    title: Option<Title>,
    author: Option<Contributor>,
    translator: Option<Contributor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contributors: Vec<ContributorEntry>,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    created: DateValue,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    issued: DateValue,
    container: Option<WorkRelation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    edition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    part_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supplement_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    printing_number: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    numbering: Vec<Numbering>,
    pages: Option<NumOrStr>,
    #[serde(alias = "URL")]
    url: Option<Url>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    accessed: Option<DateValue>,
    language: Option<LangID>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    field_languages: FieldLanguageMap,
    note: Option<RichText>,
    abstract_text: Option<RichText>,
    #[serde(alias = "DOI")]
    doi: Option<String>,
    genre: Option<String>,
    medium: Option<String>,
    status: Option<String>,
    archive_info: Option<ArchiveInfo>,
    eprint: Option<EprintInfo>,
    keywords: Option<Vec<String>>,
    original: Option<WorkRelation>,
    #[serde(flatten, default)]
    unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl From<CollectionComponentDeser> for CollectionComponent {
    fn from(raw: CollectionComponentDeser) -> Self {
        let mut component = Self {
            id: raw.id,
            r#type: raw.r#type,
            title: raw.title,
            author: raw.author,
            translator: raw.translator,
            contributors: raw.contributors,
            created: raw.created,
            issued: raw.issued,
            container: raw.container,
            volume: raw.volume,
            issue: raw.issue,
            edition: raw.edition,
            number: raw.number,
            part_number: raw.part_number,
            supplement_number: raw.supplement_number,
            printing_number: raw.printing_number,
            numbering: raw.numbering,
            pages: raw.pages,
            url: raw.url,
            accessed: raw.accessed,
            language: raw.language,
            field_languages: raw.field_languages,
            note: raw.note,
            abstract_text: raw.abstract_text,
            doi: raw.doi,
            genre: raw.genre,
            medium: raw.medium,
            status: raw.status,
            archive_info: raw.archive_info,
            eprint: raw.eprint,
            keywords: raw.keywords,
            original: raw.original,
            unknown_fields: raw.unknown_fields,
        };
        component.normalize_contributors();
        component.normalize_numbering();
        component
    }
}

crate::tolerant_enum! {
    /// Discriminates monograph-component subtypes for style-directed formatting.
    #[derive(Debug, Default, Clone, PartialEq)]
    pub enum MonographComponentType {
        /// A chapter within a book or edited volume.
        #[default]
        Chapter = "chapter",
        /// A document component that does not fit a more specific subtype.
        Document = "document"
    }
}

/// A component of a larger serial publication; for example a journal or newspaper article.
/// The parent serial is referenced by its ID.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(from = "SerialComponentDeser", rename_all = "kebab-case")]
pub struct SerialComponent {
    /// Unique identifier for this reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RefID>,
    /// Component subtype for style-directed formatting.
    pub r#type: SerialComponentType,
    /// Title of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// Author(s) of the component.
    #[serde(skip_serializing)]
    pub author: Option<Contributor>,
    /// Translator(s) of the component.
    #[serde(skip_serializing)]
    pub translator: Option<Contributor>,
    /// Unified contributor list with explicit role tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<ContributorEntry>,
    /// Creation or origination date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub created: DateValue,
    /// Publication date.
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default, skip_serializing_if = "DateValue::is_empty")]
    pub issued: DateValue,
    /// The parent work, such as a magazine, journal, or book set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<WorkRelation>,
    /// Volume number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Edition (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Generic document number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// Part number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    /// Supplement number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplement_number: Option<String>,
    /// Printing number (shorthand for numbering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printing_number: Option<String>,
    /// Numbering identifiers (e.g., volume, issue, number).
    /// Flat shorthand fields are accepted on input for authoring ergonomics and normalized
    /// into canonical `numbering` entries during deserialization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbering: Vec<Numbering>,
    /// URL for the component.
    #[serde(alias = "URL", skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    /// Date the URL was accessed.
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<DateValue>,
    /// BCP 47 language of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LangID>,
    /// Per-field language overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_languages: FieldLanguageMap,
    /// Freeform note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<RichText>,
    /// Abstract or summary of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<RichText>,
    /// DOI identifier.
    #[serde(alias = "DOI", skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// ADS bibcode identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ads_bibcode: Option<String>,
    /// Page range within the parent serial issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    /// Free-text genre descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Free-text medium descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    /// Structured archival location metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_info: Option<ArchiveInfo>,
    /// Preprint server identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eprint: Option<EprintInfo>,
    /// Keywords or subject tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// Section within the serial issue (e.g., `"A"`, `"Sports"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Publication status (e.g., `"forthcoming"`, `"ahead of print"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Date the component became publicly available (e.g., ahead-of-print date).
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_date: Option<DateValue>,
    /// Event relation associated with the component, such as a performance venue/date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<WorkRelation>,
    /// Runtime or duration of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// Work relation for reviews.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<WorkRelation>,
    /// Original publication relation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<WorkRelation>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// reference produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl SerialComponent {
    /// Fold the `author`/`translator` shorthands into the canonical `contributors`
    /// vec and re-derive the per-role views. See [`Monograph::normalize_contributors`]
    /// for why this exists. Idempotent.
    pub(crate) fn normalize_contributors(&mut self) {
        let (contributors, views) = reconcile_contributors(
            std::mem::take(&mut self.contributors),
            self.author.take(),
            None,
            self.translator.take(),
        );
        self.contributors = contributors;
        self.author = views.author;
        self.translator = views.translator;
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
struct SerialComponentDeser {
    id: Option<RefID>,
    r#type: SerialComponentType,
    title: Option<Title>,
    author: Option<Contributor>,
    translator: Option<Contributor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contributors: Vec<ContributorEntry>,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    created: DateValue,
    #[cfg_attr(feature = "bindings", specta(type = String))]
    #[serde(default)]
    issued: DateValue,
    container: Option<WorkRelation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    edition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    part_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supplement_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    printing_number: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    numbering: Vec<Numbering>,
    #[serde(alias = "URL")]
    url: Option<Url>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    accessed: Option<DateValue>,
    language: Option<LangID>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    field_languages: FieldLanguageMap,
    note: Option<RichText>,
    abstract_text: Option<RichText>,
    #[serde(alias = "DOI")]
    doi: Option<String>,
    ads_bibcode: Option<String>,
    pages: Option<String>,
    genre: Option<String>,
    medium: Option<String>,
    archive_info: Option<ArchiveInfo>,
    eprint: Option<EprintInfo>,
    keywords: Option<Vec<String>>,
    section: Option<String>,
    status: Option<String>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    available_date: Option<DateValue>,
    event: Option<WorkRelation>,
    duration: Option<String>,
    reviewed: Option<WorkRelation>,
    original: Option<WorkRelation>,
    #[serde(flatten, default)]
    unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl From<SerialComponentDeser> for SerialComponent {
    fn from(raw: SerialComponentDeser) -> Self {
        let mut component = Self {
            id: raw.id,
            r#type: raw.r#type,
            title: raw.title,
            author: raw.author,
            translator: raw.translator,
            contributors: raw.contributors,
            created: raw.created,
            issued: raw.issued,
            container: raw.container,
            volume: raw.volume,
            issue: raw.issue,
            edition: raw.edition,
            number: raw.number,
            part_number: raw.part_number,
            supplement_number: raw.supplement_number,
            printing_number: raw.printing_number,
            numbering: raw.numbering,
            url: raw.url,
            accessed: raw.accessed,
            language: raw.language,
            field_languages: raw.field_languages,
            note: raw.note,
            abstract_text: raw.abstract_text,
            doi: raw.doi,
            ads_bibcode: raw.ads_bibcode,
            pages: raw.pages,
            genre: raw.genre,
            medium: raw.medium,
            archive_info: raw.archive_info,
            eprint: raw.eprint,
            keywords: raw.keywords,
            section: raw.section,
            status: raw.status,
            available_date: raw.available_date,
            event: raw.event,
            duration: raw.duration,
            reviewed: raw.reviewed,
            original: raw.original,
            unknown_fields: raw.unknown_fields,
        };
        component.normalize_contributors();
        component.normalize_numbering();
        component
    }
}

crate::tolerant_enum! {
    /// Discriminates serial-component subtypes for style-directed formatting.
    #[derive(Debug, Default, Clone, PartialEq)]
    pub enum SerialComponentType {
        /// A peer-reviewed or editorial article in a journal, magazine, or newspaper.
        #[default]
        Article = "article",
        /// A post within an online serial (blog, news site, social feed).
        Post = "post",
        /// A review published in a serial (book review, film review, etc.).
        Review = "review"
    }
}

/// A serial publication (journal, magazine, etc.).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(from = "SerialDeser", rename_all = "kebab-case")]
pub struct Serial {
    /// Unique identifier for this reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RefID>,
    /// Serial subtype for style-directed formatting.
    pub r#type: SerialType,
    /// Title of the serial.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// Optional short form of the title for style-directed rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_title: Option<String>,
    /// The parent container for this serial (e.g., a larger series).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<WorkRelation>,
    /// Editor(s) of the serial.
    #[serde(skip_serializing)]
    pub editor: Option<Contributor>,
    /// Unified contributor list with explicit role tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<ContributorEntry>,
    /// Publisher of the serial.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,
    /// URL for the serial.
    #[serde(alias = "URL", skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    /// Date the URL was accessed.
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<DateValue>,
    /// BCP 47 language of the serial.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LangID>,
    /// Per-field language overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_languages: FieldLanguageMap,
    /// Freeform note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<RichText>,
    /// ISSN identifier.
    #[serde(alias = "ISSN", skip_serializing_if = "Option::is_none")]
    pub issn: Option<String>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// reference produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Serial {
    /// Fold the `editor` shorthand into the canonical `contributors` vec and
    /// re-derive the per-role view. See [`Monograph::normalize_contributors`]
    /// for why this exists. Idempotent.
    pub(crate) fn normalize_contributors(&mut self) {
        let (contributors, views) = reconcile_contributors(
            std::mem::take(&mut self.contributors),
            None,
            self.editor.take(),
            None,
        );
        self.contributors = contributors;
        self.editor = views.editor;
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
struct SerialDeser {
    id: Option<RefID>,
    r#type: SerialType,
    title: Option<Title>,
    short_title: Option<String>,
    container: Option<WorkRelation>,
    editor: Option<Contributor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contributors: Vec<ContributorEntry>,
    publisher: Option<Publisher>,
    #[serde(alias = "URL")]
    url: Option<Url>,
    #[cfg_attr(feature = "bindings", specta(type = Option<String>))]
    accessed: Option<DateValue>,
    language: Option<LangID>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    field_languages: FieldLanguageMap,
    note: Option<RichText>,
    #[serde(alias = "ISSN")]
    issn: Option<String>,
    #[serde(flatten, default)]
    unknown_fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl From<SerialDeser> for Serial {
    fn from(raw: SerialDeser) -> Self {
        let mut serial = Self {
            id: raw.id,
            r#type: raw.r#type,
            title: raw.title,
            short_title: raw.short_title,
            container: raw.container,
            editor: raw.editor,
            contributors: raw.contributors,
            publisher: raw.publisher,
            url: raw.url,
            accessed: raw.accessed,
            language: raw.language,
            field_languages: raw.field_languages,
            note: raw.note,
            issn: raw.issn,
            unknown_fields: raw.unknown_fields,
        };
        serial.normalize_contributors();
        serial
    }
}

impl HasNumbering for Monograph {
    fn numbering(&self) -> &[Numbering] {
        &self.numbering
    }
}

impl NormalizeNumbering for Monograph {
    fn numbering_mut(&mut self) -> &mut Vec<Numbering> {
        &mut self.numbering
    }

    fn volume_mut(&mut self) -> &mut Option<String> {
        &mut self.volume
    }

    fn issue_mut(&mut self) -> &mut Option<String> {
        &mut self.issue
    }

    fn edition_mut(&mut self) -> &mut Option<String> {
        &mut self.edition
    }

    fn number_mut(&mut self) -> &mut Option<String> {
        &mut self.number
    }

    fn part_number_mut(&mut self) -> &mut Option<String> {
        &mut self.part_number
    }

    fn supplement_number_mut(&mut self) -> &mut Option<String> {
        &mut self.supplement_number
    }

    fn printing_number_mut(&mut self) -> &mut Option<String> {
        &mut self.printing_number
    }
}

impl HasNumbering for Collection {
    fn numbering(&self) -> &[Numbering] {
        &self.numbering
    }
}

impl NormalizeNumbering for Collection {
    fn numbering_mut(&mut self) -> &mut Vec<Numbering> {
        &mut self.numbering
    }

    fn volume_mut(&mut self) -> &mut Option<String> {
        &mut self.volume
    }

    fn issue_mut(&mut self) -> &mut Option<String> {
        &mut self.issue
    }

    fn edition_mut(&mut self) -> &mut Option<String> {
        &mut self.edition
    }

    fn number_mut(&mut self) -> &mut Option<String> {
        &mut self.number
    }

    fn part_number_mut(&mut self) -> &mut Option<String> {
        &mut self.part_number
    }

    fn supplement_number_mut(&mut self) -> &mut Option<String> {
        &mut self.supplement_number
    }

    fn printing_number_mut(&mut self) -> &mut Option<String> {
        &mut self.printing_number
    }
}

impl HasNumbering for CollectionComponent {
    fn numbering(&self) -> &[Numbering] {
        &self.numbering
    }
}

impl NormalizeNumbering for CollectionComponent {
    fn numbering_mut(&mut self) -> &mut Vec<Numbering> {
        &mut self.numbering
    }

    fn volume_mut(&mut self) -> &mut Option<String> {
        &mut self.volume
    }

    fn issue_mut(&mut self) -> &mut Option<String> {
        &mut self.issue
    }

    fn edition_mut(&mut self) -> &mut Option<String> {
        &mut self.edition
    }

    fn number_mut(&mut self) -> &mut Option<String> {
        &mut self.number
    }

    fn part_number_mut(&mut self) -> &mut Option<String> {
        &mut self.part_number
    }

    fn supplement_number_mut(&mut self) -> &mut Option<String> {
        &mut self.supplement_number
    }

    fn printing_number_mut(&mut self) -> &mut Option<String> {
        &mut self.printing_number
    }
}

impl HasNumbering for SerialComponent {
    fn numbering(&self) -> &[Numbering] {
        &self.numbering
    }
}

impl NormalizeNumbering for SerialComponent {
    fn numbering_mut(&mut self) -> &mut Vec<Numbering> {
        &mut self.numbering
    }

    fn volume_mut(&mut self) -> &mut Option<String> {
        &mut self.volume
    }

    fn issue_mut(&mut self) -> &mut Option<String> {
        &mut self.issue
    }

    fn edition_mut(&mut self) -> &mut Option<String> {
        &mut self.edition
    }

    fn number_mut(&mut self) -> &mut Option<String> {
        &mut self.number
    }

    fn part_number_mut(&mut self) -> &mut Option<String> {
        &mut self.part_number
    }

    fn supplement_number_mut(&mut self) -> &mut Option<String> {
        &mut self.supplement_number
    }

    fn printing_number_mut(&mut self) -> &mut Option<String> {
        &mut self.printing_number
    }
}

/// Types of serial publications.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SerialType {
    /// An academic peer-reviewed journal.
    #[default]
    AcademicJournal,
    /// A blog or personal website updated periodically.
    Blog,
    /// A general-interest magazine.
    Magazine,
    /// A newspaper.
    Newspaper,
    /// A newsletter.
    Newsletter,
    /// A conference proceedings serial.
    Proceedings,
    /// A podcast.
    Podcast,
    /// A broadcast program (radio, television).
    BroadcastProgram,
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
    use crate::reference::types::common::NumberingType;

    #[test]
    fn monograph_part_number_normalizes_to_numbering() {
        let json = r#"{
            "type": "book",
            "title": "Test Book",
            "part-number": "II"
        }"#;
        let monograph: Monograph =
            serde_json::from_str(json).expect("monograph should deserialize");
        assert!(monograph.part_number.is_none());
        assert!(
            monograph
                .numbering
                .iter()
                .any(|n| n.r#type == NumberingType::Part && n.value == "II")
        );
    }

    #[test]
    fn serial_component_supplement_number_normalizes_to_numbering() {
        let json = r#"{
            "type": "article",
            "title": "Test Article",
            "supplement-number": "S1"
        }"#;
        let component: SerialComponent =
            serde_json::from_str(json).expect("serial component should deserialize");
        assert!(component.supplement_number.is_none());
        assert!(
            component
                .numbering
                .iter()
                .any(|n| n.r#type == NumberingType::Supplement && n.value == "S1")
        );
    }

    #[test]
    fn monograph_printing_number_normalizes_to_numbering() {
        let json = r#"{
            "type": "book",
            "title": "Test Book",
            "printing-number": "3rd printing"
        }"#;
        let monograph: Monograph =
            serde_json::from_str(json).expect("monograph should deserialize");
        assert!(monograph.printing_number.is_none());
        assert!(
            monograph
                .numbering
                .iter()
                .any(|n| n.r#type == NumberingType::Printing && n.value == "3rd printing")
        );
    }

    #[test]
    fn collection_component_printing_number_normalizes_to_numbering() {
        let json = r#"{
            "type": "chapter",
            "title": "Test Chapter",
            "printing-number": "2"
        }"#;
        let component: CollectionComponent =
            serde_json::from_str(json).expect("collection component should deserialize");
        assert!(component.printing_number.is_none());
        assert!(
            component
                .numbering
                .iter()
                .any(|n| n.r#type == NumberingType::Printing && n.value == "2")
        );
    }
}
