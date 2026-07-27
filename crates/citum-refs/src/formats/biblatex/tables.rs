/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Declarative BibLaTeX ↔ Citum mapping tables.
//!
//! [`BIBLATEX_ENTRY_TYPES`] and [`BIBLATEX_FIELDS`] are the source of truth
//! for the generated `docs/reference/BIBLATEX_MAPPING.md` (via
//! `docs/schemas/type-map.json`, emitted by `citum schema`; see
//! [`biblatex_entry_type_descriptors`] and [`biblatex_field_descriptors`]).
//!
//! `BIBLATEX_FIELDS` is declarative only: it documents what the manual's
//! §2.2.1 data-type vocabulary says about each field and where Citum's
//! extraction (in `super::mapping`) currently sends it, but extraction still
//! runs through the untyped `field_str`/`rich_field_str` closures and the
//! handful of typed `biblatex` crate accessors it already calls — this table
//! does not drive extraction. `BiblatexDataType` is not consulted at
//! extraction time (no `match datatype { ... }` dispatch); the two remaining
//! narrow fixes it motivated -- `Chunk::Math` no longer discarded, and
//! `LiteralList` fields (`publisher`/`institution`/`organization`/`school`/
//! `location`) joined with `"; "` instead of leaking BibLaTeX's `and`
//! separator -- were applied
//! directly in `mapping.rs` (`chunk_to_string`/`literal_list_str`) rather
//! than through a generic per-datatype dispatcher, since only those two
//! datatypes needed different handling from plain concatenation. `Date`/
//! `Range`/`Integer` fields deliberately still use hand-rolled string
//! extraction, not the crate's typed accessors (`entry.date()`,
//! `entry.pages()`, ...): those normalize their input, which would change
//! rendered output for already-mapped fields and risk the gb7714 fidelity
//! baseline. See bean csl26-qtur's follow-ups for the fuller typed-dispatch
//! refactor this stops short of.
//!
//! [`BIBLATEX_ENTRY_TYPES`] **does** drive dispatch — [`biblatex_entry_mapping`]
//! replaces the previous inline `match` in
//! `super::mapping::input_reference_from_biblatex`. Some rows intentionally
//! change the native reference class to preserve BibLaTeX semantics more
//! accurately; the generated mapping reference documents those decisions.
//!
//! The `datatype`/`crate_accessor` columns are populated from the BibLaTeX
//! manual §2.2.1/§2.2.2, not inferred from the `biblatex` crate — the crate
//! returns bare `ChunksRef` for several fields the manual types more
//! narrowly. Where the two disagree, the manual's datatype wins and the
//! crate's actual accessor is recorded separately in `crate_accessor`.

use citum_schema::reference::types::{CollectionType, MonographType};

/// How a BibLaTeX entry type is converted into an `InputReference`.
#[derive(Debug)]
pub(super) enum BiblatexBuilder {
    /// A flat `Monograph` of the given `MonographType`, via
    /// `mapping::biblatex_monograph`.
    Monograph(MonographType),
    /// `mapping::build_inbook_reference` — a `CollectionComponent` embedded
    /// in a synthesized parent `Collection`.
    Inbook,
    /// `mapping::build_article_reference` — a `SerialComponent` embedded in
    /// a synthesized parent `Serial`.
    Article,
    /// `mapping::build_collection_reference` — a standalone `Collection` of
    /// the given `CollectionType`.
    Collection(CollectionType),
    /// `mapping::build_patent_reference` — a standalone `Patent` when
    /// `number` is nonblank, otherwise the generic document fallback.
    Patent,
    /// `mapping::build_dataset_reference` — a standalone `Dataset`.
    Dataset,
    /// `mapping::build_software_reference` — a standalone `Software`.
    Software,
    /// `mapping::build_standard_reference` — a standalone `Standard` when
    /// `number` is nonblank, otherwise the generic document fallback.
    Standard,
}

impl BiblatexBuilder {
    /// Stable name for this builder, for the generated doc/JSON.
    fn name(&self) -> &'static str {
        match self {
            Self::Monograph(_) => "monograph",
            Self::Inbook => "inbook",
            Self::Article => "article",
            Self::Collection(_) => "collection",
            Self::Patent => "patent",
            Self::Dataset => "dataset",
            Self::Software => "software",
            Self::Standard => "standard",
        }
    }
}

/// One row of the BibLaTeX entry-type → Citum reference-shape mapping.
#[derive(Debug)]
pub(super) struct BiblatexEntryMapping {
    /// The lowercased, alias-resolved BibLaTeX entry type. Known types use
    /// `to_biblatex()` canonicalization, so `phdthesis`/`mastersthesis`/
    /// `techreport` arrive as `thesis`/`report`; `EntryType::Unknown(raw)`
    /// uses its raw payload so non-core rows such as `standard` remain
    /// reachable.
    pub(super) entry_type: &'static str,
    pub(super) builder: BiblatexBuilder,
    /// Rationale surfaced in the generated doc; `None` when the mapping is
    /// unremarkable.
    pub(super) note: Option<&'static str>,
}

/// Explicit BibLaTeX entry-type mappings. Order does not affect dispatch
/// (lookup is by exact match), but groups entries the way the original
/// `match` did, for readability.
///
/// Every entry type not listed here falls back to [`BIBLATEX_FALLBACK`].
pub(super) const BIBLATEX_ENTRY_TYPES: &[BiblatexEntryMapping] = &[
    BiblatexEntryMapping {
        entry_type: "book",
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "mvbook",
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: Some(
            "Multi-volume book; volume-level structure is not yet modeled distinctly from a single-volume book.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "collection",
        builder: BiblatexBuilder::Collection(CollectionType::EditedBook),
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "mvcollection",
        builder: BiblatexBuilder::Collection(CollectionType::EditedBook),
        note: Some(
            "Multi-volume collection; volume-level structure is not yet modeled distinctly.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "manual",
        builder: BiblatexBuilder::Monograph(MonographType::Manual),
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "report",
        builder: BiblatexBuilder::Monograph(MonographType::Report),
        note: Some(
            "`techreport` aliases to `report` in the `biblatex` crate before this table sees it.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "thesis",
        builder: BiblatexBuilder::Monograph(MonographType::Thesis),
        note: Some(
            "`phdthesis`/`mastersthesis` alias to `thesis` in the `biblatex` crate before this table sees it.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "online",
        builder: BiblatexBuilder::Monograph(MonographType::Webpage),
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "unpublished",
        builder: BiblatexBuilder::Monograph(MonographType::Manuscript),
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "proceedings",
        builder: BiblatexBuilder::Collection(CollectionType::Proceedings),
        note: Some(
            "A journal-like recurring proceedings (ISSN, cited as vol(issue): pages) arrives as `@article` with a `journaltitle` instead, and is already routed to `Serial` — `@proceedings` is BibLaTeX's edited-volume case (ISBN), the same distinction the manual draws between `@article` and `@inproceedings`. `SerialType::Proceedings` exists for the journal-like case but has no BibLaTeX producer for exactly this reason.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "mvproceedings",
        builder: BiblatexBuilder::Collection(CollectionType::Proceedings),
        note: Some("Same as `proceedings`."),
    },
    BiblatexEntryMapping {
        entry_type: "inbook",
        builder: BiblatexBuilder::Inbook,
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "incollection",
        builder: BiblatexBuilder::Inbook,
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "inproceedings",
        builder: BiblatexBuilder::Inbook,
        note: Some(
            "`eventtitle`/`venue`/`eventdate` (the conference itself, distinct from `booktitle`, the proceedings volume) map onto the synthesized parent `Collection`'s `event` field (an embedded `Event`), the same shape the CSL-JSON `paper-conference` path uses. The parent `Collection` is `CollectionType::Proceedings` here, vs. `EditedBook` for `inbook`/`incollection`.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "article",
        builder: BiblatexBuilder::Article,
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "patent",
        builder: BiblatexBuilder::Patent,
        note: Some(
            "`number` is required by the native `Patent` shape. Missing, empty, or whitespace-only values retain the generic `Document` fallback rather than creating an invalid patent.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "dataset",
        builder: BiblatexBuilder::Dataset,
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "software",
        builder: BiblatexBuilder::Software,
        note: None,
    },
    BiblatexEntryMapping {
        entry_type: "standard",
        builder: BiblatexBuilder::Standard,
        note: Some(
            "Not a core BibLaTeX/BibTeX entry type -- arrives as `EntryType::Unknown(\"standard\")`. Reachable here because entry-type dispatch reads the raw string for unknown types. `number` is required by the native `Standard` shape; missing, empty, or whitespace-only values retain the generic `Document` fallback.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "periodical",
        builder: BiblatexBuilder::Monograph(MonographType::Document),
        note: Some(
            "A complete periodical issue, represented by the existing `Document` compatibility contract with canonical genre `periodical` so its issued date and back-mapped type survive. This is intentionally lossy: the current model cannot express issue → journal hierarchy or retain ISSN canonically.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "reference",
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: Some("A work of reference (encyclopedia, dictionary); same shape as `@book`."),
    },
    BiblatexEntryMapping {
        entry_type: "mvreference",
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: Some("Same as `reference`."),
    },
    BiblatexEntryMapping {
        entry_type: "inreference",
        builder: BiblatexBuilder::Inbook,
        note: Some(
            "An entry in a work of reference. Uses the collection-component chapter shape with canonical genre `entry`, which back-maps to the `entry` reference type.",
        ),
    },
];

/// Fallback row for every BibLaTeX entry type not listed in
/// [`BIBLATEX_ENTRY_TYPES`] (e.g. `booklet`, `misc`, `bookinbook`,
/// `suppbook`, `suppperiodical`).
///
/// Kept as a named `const` rather than folded into dispatch as a bare `_`
/// arm, purely so it renders as an explicit row in the generated
/// `docs/reference/BIBLATEX_MAPPING.md` table instead of being invisible.
pub(super) const BIBLATEX_FALLBACK: BiblatexEntryMapping = BiblatexEntryMapping {
    entry_type: "*",
    builder: BiblatexBuilder::Monograph(MonographType::Document),
    note: Some(
        "Fallback for every entry type with no dedicated builder above. `booklet`/`bookinbook`/`suppbook`/`suppperiodical` are candidates for their own builder rows in a future pass.",
    ),
};

/// Look up the mapping row for a canonicalized (lowercased, alias-resolved)
/// BibLaTeX entry type, falling back to [`BIBLATEX_FALLBACK`] for anything
/// not explicitly listed. This is the sole dispatch point
/// `mapping::input_reference_from_biblatex` uses.
pub(super) fn biblatex_entry_mapping(entry_type: &str) -> &'static BiblatexEntryMapping {
    BIBLATEX_ENTRY_TYPES
        .iter()
        .find(|row| row.entry_type == entry_type)
        .unwrap_or(&BIBLATEX_FALLBACK)
}

/// BibLaTeX manual §2.2.1 data types, used to classify each field in
/// [`BIBLATEX_FIELDS`].
#[derive(Debug, Clone, Copy)]
pub(super) enum BiblatexDataType {
    /// A person or institution name list (`author`, `editor`, `translator`,
    /// and the editorial sub-role fields).
    Name,
    /// Free text, possibly with embedded markup (`title`, `publisher`).
    Literal,
    /// A semicolon/`and`-separated list of literals (`publisher`,
    /// `location`, `organization` may each name more than one entity).
    LiteralList,
    /// An identifier string with no markup (`isbn`, `doi`).
    Verbatim,
    /// A URI (`url`).
    Uri,
    /// A whole number (`volume`, `edition`).
    Integer,
    /// A page/number range (`pages`).
    Range,
    /// An EDTF-like date (`date`, `urldate`).
    Date,
    /// A closed-vocabulary keyword (`langid`, `pagination`, `type`).
    Key,
    /// A fixed-pattern code (`gender`).
    Pattern,
    /// A comma-or-semicolon-separated free-form list (`keywords`).
    SeparatedValue,
    /// A reference to another entry's key (`crossref`, `xdata`, `related`).
    EntryKey,
}

/// Where a BibLaTeX field's value ends up in Citum's `InputReference`, or
/// that nothing currently reads it.
#[derive(Debug, Clone, Copy)]
pub(super) enum BiblatexFieldTarget {
    /// The Citum field/path this BibLaTeX field is currently folded into.
    Mapped(&'static str),
    /// No current extraction path reads this field.
    Unmapped,
}

/// One row of the BibLaTeX field → Citum field mapping.
#[derive(Debug)]
pub(super) struct BiblatexFieldMapping {
    /// The BibLaTeX field name as it appears in `.bib` source.
    pub(super) field: &'static str,
    pub(super) datatype: BiblatexDataType,
    /// The typed accessor the `biblatex` crate exposes for this field (e.g.
    /// `"entry.date()"`), if any. Documents what typed extraction is
    /// *available*, not necessarily what Citum's extraction *currently
    /// calls* — see `note`.
    pub(super) crate_accessor: Option<&'static str>,
    pub(super) target: BiblatexFieldTarget,
    /// Rationale; `None` when the mapping is unremarkable.
    pub(super) note: Option<&'static str>,
}

/// Declarative field table. Mapped rows document today's extraction path
/// (still the untyped `field_str`/`rich_field_str` closures for most
/// fields); `Unmapped` rows are the remaining gap list.
pub(super) const BIBLATEX_FIELDS: &[BiblatexFieldMapping] = &[
    BiblatexFieldMapping {
        field: "title",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.title()"),
        target: BiblatexFieldTarget::Mapped("title"),
        note: Some(
            "Read via `rich_field_str`, not the crate's `title()` accessor: converts citeproc-js HTML rich-text markup (e.g. `<span class=\"nocase\">`) to Djot.",
        ),
    },
    BiblatexFieldMapping {
        field: "subtitle",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.subtitle()"),
        target: BiblatexFieldTarget::Mapped("title.sub"),
        note: Some("Combined with `title` into a `Title::Structured` when both are present."),
    },
    BiblatexFieldMapping {
        field: "booktitle",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.book_title()"),
        target: BiblatexFieldTarget::Mapped(
            "container.title (inbook/incollection/inproceedings only)",
        ),
        note: None,
    },
    BiblatexFieldMapping {
        field: "journaltitle",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.journal_title()"),
        target: BiblatexFieldTarget::Mapped("container.title (article only)"),
        note: Some("Falls back to the BibTeX alias `journal` when absent."),
    },
    BiblatexFieldMapping {
        field: "date",
        datatype: BiblatexDataType::Date,
        crate_accessor: Some("entry.date()"),
        target: BiblatexFieldTarget::Mapped("issued"),
        note: Some(
            "Extraction reads `field_str(\"date\")` directly rather than the crate's typed `Date` accessor, so the value is stored as a raw string handed to `DateValue::new` rather than a parsed EDTF value.",
        ),
    },
    BiblatexFieldMapping {
        field: "urldate",
        datatype: BiblatexDataType::Date,
        crate_accessor: Some("entry.url_date()"),
        target: BiblatexFieldTarget::Mapped("accessed"),
        note: Some("Same raw-string caveat as `date`."),
    },
    BiblatexFieldMapping {
        field: "publisher",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.publisher()"),
        target: BiblatexFieldTarget::Mapped("publisher.name"),
        note: Some(
            "biblatex `publisher` is an `and`-separated literal list (multiple publishers). `literal_list_str` splits and rejoins with `\"; \"`, but `Publisher.name` is a single `MultilingualString` -- a genuine multi-publisher entry still collapses to one string; only the join delimiter changed (`\"; \"` instead of leaking the literal `and`), not the underlying single-valued field.",
        ),
    },
    BiblatexFieldMapping {
        field: "institution",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.institution()"),
        target: BiblatexFieldTarget::Mapped("publisher.name (fallback)"),
        note: Some(
            "Falls back to `organization`, then `school`, when `publisher` is absent. Same list-join handling as `publisher`.",
        ),
    },
    BiblatexFieldMapping {
        field: "organization",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.fields.get(\"organization\") => Vec<Chunks>"),
        target: BiblatexFieldTarget::Mapped("publisher.name (fallback)"),
        note: Some("Same list-join handling as `publisher`."),
    },
    BiblatexFieldMapping {
        field: "school",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.school()"),
        target: BiblatexFieldTarget::Mapped("publisher.name (fallback)"),
        note: Some("Same list-join handling as `publisher`."),
    },
    BiblatexFieldMapping {
        field: "location",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.location()"),
        target: BiblatexFieldTarget::Mapped("publisher.place"),
        note: Some(
            "Alias of `address`. Same list-join handling as `publisher`, and the same single-valued-field caveat (`Publisher.place` is a single `Place`).",
        ),
    },
    BiblatexFieldMapping {
        field: "url",
        datatype: BiblatexDataType::Uri,
        crate_accessor: Some("entry.url()"),
        target: BiblatexFieldTarget::Mapped("url"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "isbn",
        datatype: BiblatexDataType::Verbatim,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("isbn"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "doi",
        datatype: BiblatexDataType::Verbatim,
        crate_accessor: Some("entry.doi()"),
        target: BiblatexFieldTarget::Mapped("doi"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "note",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("note"),
        note: Some(
            "Extracted via untyped `field_str`, not `rich_field_str` — unlike `title`/`abstract`, embedded rich-text markup in `note` is not converted to Djot.",
        ),
    },
    BiblatexFieldMapping {
        field: "abstract",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("abstract-text"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "keywords",
        datatype: BiblatexDataType::SeparatedValue,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("keywords"),
        note: Some(
            "Split on `,` for Monograph, Collection, Patent, Dataset, Software, and Standard outputs. CollectionComponent and SerialComponent builders do not currently retain keywords.",
        ),
    },
    BiblatexFieldMapping {
        field: "edition",
        datatype: BiblatexDataType::Integer,
        crate_accessor: Some("entry.edition()"),
        target: BiblatexFieldTarget::Mapped("numbering[Edition]"),
        note: Some("Read via untyped `field_str`, not the crate's `PermissiveType<i64>` accessor."),
    },
    BiblatexFieldMapping {
        field: "number",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped(
            "numbering[Report|Number|Volume|Issue], context-dependent",
        ),
        note: Some(
            "Overloaded by builder: `Report` on `@report`, `Number` on other monographs, `Volume` on inbook/incollection/inproceedings' parent, `Issue` on `@article`.",
        ),
    },
    BiblatexFieldMapping {
        field: "volume",
        datatype: BiblatexDataType::Integer,
        crate_accessor: Some("entry.volume()"),
        target: BiblatexFieldTarget::Mapped("numbering[Volume] (article only)"),
        note: Some("Read via untyped `field_str`, not the crate's `PermissiveType<i64>` accessor."),
    },
    BiblatexFieldMapping {
        field: "pages",
        datatype: BiblatexDataType::Range,
        crate_accessor: Some("entry.pages()"),
        target: BiblatexFieldTarget::Mapped("pages"),
        note: Some(
            "Read via untyped `field_str` as a raw string; the crate's typed `PermissiveType<Vec<Range<u32>>>` accessor is not used, so a parsed page range is not available to extraction.",
        ),
    },
    BiblatexFieldMapping {
        field: "issn",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("container.issn (article only)"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "bibcode",
        datatype: BiblatexDataType::Verbatim,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("ads-bibcode"),
        note: Some(
            "Not a BibLaTeX manual field; a nonstandard extension some astronomy-adjacent exporters emit.",
        ),
    },
    BiblatexFieldMapping {
        field: "version",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Mapped("version"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "type",
        datatype: BiblatexDataType::Key,
        crate_accessor: Some("entry.type_()"),
        target: BiblatexFieldTarget::Mapped("genre"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "langid",
        datatype: BiblatexDataType::Key,
        crate_accessor: Some("entry.langid()"),
        target: BiblatexFieldTarget::Mapped("language"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "language",
        datatype: BiblatexDataType::Key,
        crate_accessor: Some("entry.language()"),
        target: BiblatexFieldTarget::Mapped("language (fallback)"),
        note: Some("Falls back from `langid` when absent."),
    },
    BiblatexFieldMapping {
        field: "author",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.author()"),
        target: BiblatexFieldTarget::Mapped("author / contributors[author]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "editor",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.editors()"),
        target: BiblatexFieldTarget::Mapped("editor / contributors[editor]"),
        note: Some(
            "`entry.editors()` also returns `editora`/`editorb`/`editorc` and each group's `EditorType` (`editortype` etc). Only `EditorType::Editor`-tagged groups become the `editor` shorthand; `Compiler`/`Director` groups are tagged `contributors[compiler]`/`contributors[director]`, and `Founder`/`Continuator`/`Redactor`/`Reviser`/`Collaborator`/`Organizer` (no dedicated `ContributorRole` variant) degrade to `contributors[Unknown(<name>)]`, which still round-trips and is selectable by a style as a custom role.",
        ),
    },
    BiblatexFieldMapping {
        field: "translator",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.translator()"),
        target: BiblatexFieldTarget::Mapped("translator / contributors[translator]"),
        note: None,
    },
    // --- Conversion-breadth fields completed by bean csl26-11h2 ---
    BiblatexFieldMapping {
        field: "eprint",
        datatype: BiblatexDataType::Verbatim,
        crate_accessor: Some("entry.eprint()"),
        target: BiblatexFieldTarget::Mapped("eprint.id"),
        note: Some(
            "A nonblank identifier populates `EprintInfo` on Monograph, CollectionComponent, and SerialComponent outputs. A missing `eprinttype` is represented by an empty server. Separately flips the entry's `MonographType` to `Preprint`, but only when `eprint` is nonblank and there is no container signal: an `@article` with no `journaltitle`/`journal`, or a `misc`/`unpublished`/`online`/fallback entry. Other output classes do not retain eprint metadata.",
        ),
    },
    BiblatexFieldMapping {
        field: "eprinttype",
        datatype: BiblatexDataType::Key,
        crate_accessor: Some("entry.eprint_type()"),
        target: BiblatexFieldTarget::Mapped("eprint.server"),
        note: Some("Alias of `archiveprefix`. Lowercased on extraction. See `eprint`."),
    },
    BiblatexFieldMapping {
        field: "eprintclass",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.eprint_class()"),
        target: BiblatexFieldTarget::Mapped("eprint.class"),
        note: Some("Alias of `primaryclass`. See `eprint`."),
    },
    BiblatexFieldMapping {
        field: "series",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.series()"),
        target: BiblatexFieldTarget::Mapped("container[…].container (collection-title)"),
        note: Some(
            "Reuses the CSL-JSON conversion path's shape for a `collection-title` (`relation_collection_title`): an embedded, title-only `Collection` wrapping the series name. For `@book`/etc. (no intermediate container-title), wraps in a title-less parent first, matching the CSL-JSON path's identical `container-title`-less case. For `@incollection`/`@inproceedings`/`@article` the series attaches directly to the already-synthesized parent Collection/Serial. A `number` alongside `series` becomes `NumberingType::Volume` (volume-in-series) rather than a generic document number.",
        ),
    },
    BiblatexFieldMapping {
        field: "eventtitle",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.eventtitle()"),
        target: BiblatexFieldTarget::Mapped("container.event.title"),
        note: Some(
            "The conference/event name for `@inproceedings`, distinct from `booktitle` (the proceedings volume). Only read for `@inproceedings`; see the entry-type table.",
        ),
    },
    BiblatexFieldMapping {
        field: "venue",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.venue()"),
        target: BiblatexFieldTarget::Mapped("container.event.location"),
        note: Some("Same event shape as `eventtitle`."),
    },
    BiblatexFieldMapping {
        field: "eventdate",
        datatype: BiblatexDataType::Date,
        crate_accessor: Some("entry.event_date()"),
        target: BiblatexFieldTarget::Mapped("container.event.date"),
        note: Some("Same event shape as `eventtitle`."),
    },
    BiblatexFieldMapping {
        field: "chapter",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.chapter()"),
        target: BiblatexFieldTarget::Mapped("numbering[chapter]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "afterword",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.afterword()"),
        target: BiblatexFieldTarget::Mapped("contributors[afterword-author]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "annotator",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.annotator()"),
        target: BiblatexFieldTarget::Mapped("contributors[annotator]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "commentator",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.commentator()"),
        target: BiblatexFieldTarget::Mapped("contributors[commentator]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "foreword",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.foreword()"),
        target: BiblatexFieldTarget::Mapped("contributors[foreword-author]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "introduction",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.introduction()"),
        target: BiblatexFieldTarget::Mapped("contributors[introduction-author]"),
        note: None,
    },
    BiblatexFieldMapping {
        field: "holder",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.holder()"),
        target: BiblatexFieldTarget::Mapped("Patent.assignee"),
        note: Some("Patent holder/assignee. Only read for `@patent`."),
    },
    BiblatexFieldMapping {
        field: "gender",
        datatype: BiblatexDataType::Pattern,
        crate_accessor: Some("entry.gender()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "Documented divergence, not a pending mapping: biblatex `Gender` has six values conflating number × gender (singular/plural × feminine/masculine/neuter); Citum's `ContributorGender` has four (masculine/feminine/neuter/common), no number axis, and a `Common` value biblatex lacks. See docs/specs/GENDERED_LOCALE_TERMS.md.",
        ),
    },
    BiblatexFieldMapping {
        field: "crossref",
        datatype: BiblatexDataType::EntryKey,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "Resolved transparently by `biblatex::Bibliography::parse` before Citum's mapping runs — fields inherited from the crossref'd parent are merged into `entry.fields` — so this is not a `WorkRelation` Citum needs to construct itself, unlike `related`/`relatedtype` below.",
        ),
    },
    BiblatexFieldMapping {
        field: "xdata",
        datatype: BiblatexDataType::EntryKey,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "Field-splicing mechanism, resolved and removed by `biblatex::Bibliography::parse` before Citum's mapping runs, like `crossref`.",
        ),
    },
    BiblatexFieldMapping {
        field: "related",
        datatype: BiblatexDataType::EntryKey,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "Typed relation to another entry (paired with `relatedtype`: `multivolume`/`origpub`/`reprint`/`translationof`/`reviewof`/…), *not* resolved by the crate. Closest BibLaTeX prior art to `WorkRelation`. See docs/specs/ORIGINAL_PUBLICATION_RELATION_SUPPORT.md.",
        ),
    },
    BiblatexFieldMapping {
        field: "relatedtype",
        datatype: BiblatexDataType::Key,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Names the relation kind for `related`; see `related`."),
    },
];

/// Serializable descriptor for one `BiblatexEntryMapping` row, emitted into
/// `docs/schemas/type-map.json` by `citum schema` and rendered to markdown by
/// `scripts/build-data-model-reference.js`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BiblatexEntryTypeDescriptor {
    /// The lowercased, alias-resolved BibLaTeX entry type, or `"*"` for the
    /// fallback row.
    pub entry_type: &'static str,
    /// Which builder converts this entry type: `"monograph"`, `"inbook"`,
    /// `"article"`, `"collection"`, `"patent"`, `"dataset"`, `"software"`,
    /// or `"standard"`.
    pub builder: &'static str,
    /// The resulting `MonographType` wire value, when `builder` is
    /// `"monograph"`.
    pub monograph_type: Option<String>,
    /// The resulting `CollectionType` wire value, when `builder` is
    /// `"collection"`.
    pub collection_type: Option<String>,
    /// Reserved for compatibility with generated type-map consumers.
    pub serial_type: Option<String>,
    /// Rationale; `None` when the mapping is unremarkable.
    pub note: Option<&'static str>,
}

impl From<&BiblatexEntryMapping> for BiblatexEntryTypeDescriptor {
    fn from(row: &BiblatexEntryMapping) -> Self {
        let monograph_type = match &row.builder {
            BiblatexBuilder::Monograph(mono_type) => Some(mono_type.as_str().to_string()),
            BiblatexBuilder::Inbook
            | BiblatexBuilder::Article
            | BiblatexBuilder::Collection(_)
            | BiblatexBuilder::Patent
            | BiblatexBuilder::Dataset
            | BiblatexBuilder::Software
            | BiblatexBuilder::Standard => None,
        };
        let collection_type = match &row.builder {
            BiblatexBuilder::Collection(collection_type) => {
                Some(collection_type.as_str().to_string())
            }
            BiblatexBuilder::Monograph(_)
            | BiblatexBuilder::Inbook
            | BiblatexBuilder::Article
            | BiblatexBuilder::Patent
            | BiblatexBuilder::Dataset
            | BiblatexBuilder::Software
            | BiblatexBuilder::Standard => None,
        };
        Self {
            entry_type: row.entry_type,
            builder: row.builder.name(),
            monograph_type,
            collection_type,
            serial_type: None,
            note: row.note,
        }
    }
}

/// All entry-type mapping rows, including the `BIBLATEX_FALLBACK` row, as
/// serializable descriptors.
#[must_use]
pub fn biblatex_entry_type_descriptors() -> Vec<BiblatexEntryTypeDescriptor> {
    BIBLATEX_ENTRY_TYPES
        .iter()
        .chain(std::iter::once(&BIBLATEX_FALLBACK))
        .map(BiblatexEntryTypeDescriptor::from)
        .collect()
}

impl BiblatexDataType {
    /// Stable name for this datatype, for the generated doc/JSON.
    fn name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Literal => "literal",
            Self::LiteralList => "literal-list",
            Self::Verbatim => "verbatim",
            Self::Uri => "uri",
            Self::Integer => "integer",
            Self::Range => "range",
            Self::Date => "date",
            Self::Key => "key",
            Self::Pattern => "pattern",
            Self::SeparatedValue => "separated-value",
            Self::EntryKey => "entrykey",
        }
    }
}

/// Serializable descriptor for one `BiblatexFieldMapping` row, emitted into
/// `docs/schemas/type-map.json` by `citum schema` and rendered to markdown by
/// `scripts/build-data-model-reference.js`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BiblatexFieldDescriptor {
    /// The BibLaTeX field name as it appears in `.bib` source.
    pub field: &'static str,
    /// The BibLaTeX manual §2.2.1 data type, as a wire string (e.g.
    /// `"literal-list"`).
    pub datatype: &'static str,
    /// The typed accessor the `biblatex` crate exposes for this field, if
    /// any, regardless of whether Citum's extraction currently calls it.
    pub crate_accessor: Option<&'static str>,
    /// The Citum field/path this maps to, or `None` when nothing currently
    /// reads it.
    pub target: Option<&'static str>,
    /// Rationale; `None` when the mapping is unremarkable.
    pub note: Option<&'static str>,
}

impl From<&BiblatexFieldMapping> for BiblatexFieldDescriptor {
    fn from(row: &BiblatexFieldMapping) -> Self {
        let target = match row.target {
            BiblatexFieldTarget::Mapped(target) => Some(target),
            BiblatexFieldTarget::Unmapped => None,
        };
        Self {
            field: row.field,
            datatype: row.datatype.name(),
            crate_accessor: row.crate_accessor,
            target,
            note: row.note,
        }
    }
}

/// All field mapping rows as serializable descriptors.
#[must_use]
pub fn biblatex_field_descriptors() -> Vec<BiblatexFieldDescriptor> {
    BIBLATEX_FIELDS
        .iter()
        .map(BiblatexFieldDescriptor::from)
        .collect()
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

    #[test]
    fn given_known_entry_type_when_looked_up_then_returns_its_row() {
        let row = biblatex_entry_mapping("report");

        assert_eq!(row.entry_type, "report");
        assert!(matches!(
            row.builder,
            BiblatexBuilder::Monograph(MonographType::Report)
        ));
    }

    #[test]
    fn given_unknown_entry_type_when_looked_up_then_returns_the_fallback_row() {
        let row = biblatex_entry_mapping("booklet");

        assert_eq!(row.entry_type, "*");
        assert!(matches!(
            row.builder,
            BiblatexBuilder::Monograph(MonographType::Document)
        ));
    }

    #[test]
    fn given_entry_type_table_when_described_then_fallback_row_is_included_last() {
        let descriptors = biblatex_entry_type_descriptors();

        assert_eq!(descriptors.len(), BIBLATEX_ENTRY_TYPES.len() + 1);
        assert_eq!(descriptors.last().expect("non-empty").entry_type, "*");
    }

    #[test]
    fn given_field_table_when_described_then_row_count_matches() {
        let descriptors = biblatex_field_descriptors();

        assert_eq!(descriptors.len(), BIBLATEX_FIELDS.len());
    }
}
