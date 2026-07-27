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
//! does not drive extraction. Rewiring extraction to dispatch on
//! [`BiblatexDataType`] is a follow-up (see bean csl26-qtur's follow-ups),
//! since it changes conversion output (typed dates, real page ranges,
//! list-valued publishers) and so does not belong in this refactor.
//!
//! [`BIBLATEX_ENTRY_TYPES`] **does** drive dispatch — [`biblatex_entry_mapping`]
//! replaces the previous inline `match` in
//! `super::mapping::input_reference_from_biblatex`. This part of the
//! refactor is behavior-preserving: every row reproduces exactly the
//! `MonographType`/builder the old `match` produced for that entry type.
//!
//! The `datatype`/`crate_accessor` columns are populated from the BibLaTeX
//! manual §2.2.1/§2.2.2, not inferred from the `biblatex` crate — the crate
//! returns bare `ChunksRef` for several fields the manual types more
//! narrowly. Where the two disagree, the manual's datatype wins and the
//! crate's actual accessor is recorded separately in `crate_accessor`.

use citum_schema::reference::types::MonographType;

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
}

impl BiblatexBuilder {
    /// Stable name for this builder, for the generated doc/JSON.
    fn name(&self) -> &'static str {
        match self {
            Self::Monograph(_) => "monograph",
            Self::Inbook => "inbook",
            Self::Article => "article",
        }
    }
}

/// One row of the BibLaTeX entry-type → Citum reference-shape mapping.
#[derive(Debug)]
pub(super) struct BiblatexEntryMapping {
    /// The lowercased, alias-resolved BibLaTeX entry type
    /// (`entry.entry_type.to_biblatex().to_string().to_lowercase()`) — e.g.
    /// `phdthesis`/`mastersthesis`/`techreport` never appear here because the
    /// `biblatex` crate already canonicalizes them (to `thesis`/`report`)
    /// before dispatch sees them.
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
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: Some(
            "Not routed to the `Collection` reference class despite the name — a bare `@collection` carries no per-chapter structure to justify one. Whether it should be is a modeling decision, not a mapping gap.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "mvcollection",
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: Some("Same as `collection`."),
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
        builder: BiblatexBuilder::Monograph(MonographType::Book),
        note: Some("Not routed to `Collection`; see the `collection` row above."),
    },
    BiblatexEntryMapping {
        entry_type: "mvproceedings",
        builder: BiblatexBuilder::Monograph(MonographType::Book),
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
            "`eventtitle`/`venue` (the conference itself, distinct from `booktitle`, the proceedings volume) are not read — no schema slot on `CollectionComponent` today. See `BIBLATEX_FIELDS`.",
        ),
    },
    BiblatexEntryMapping {
        entry_type: "article",
        builder: BiblatexBuilder::Article,
        note: None,
    },
];

/// Fallback row for every BibLaTeX entry type not listed in
/// [`BIBLATEX_ENTRY_TYPES`] (e.g. `booklet`, `misc`, `patent`, `dataset`,
/// `software`, `standard`, `periodical`, `reference`, `inreference`,
/// `bookinbook`, `suppbook`, `suppperiodical`, `mvreference`).
///
/// Kept as a named `const` rather than folded into dispatch as a bare `_`
/// arm, purely so it renders as an explicit row in the generated
/// `docs/reference/BIBLATEX_MAPPING.md` table instead of being invisible.
pub(super) const BIBLATEX_FALLBACK: BiblatexEntryMapping = BiblatexEntryMapping {
    entry_type: "*",
    builder: BiblatexBuilder::Monograph(MonographType::Document),
    note: Some(
        "Fallback for every entry type with no dedicated builder above. Each of these is a candidate for its own builder/`ReferenceClass` — `Patent`, `Dataset`, `Software`, and `Standard` already exist as standalone reference classes in citum-schema-data::reference::types::specialized, just unused by BibLaTeX conversion today.",
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
/// fields); `Unmapped` rows are the gap list — this *is* the enumerated form
/// of bean csl26-11h2's open items.
pub(super) const BIBLATEX_FIELDS: &[BiblatexFieldMapping] = &[
    BiblatexFieldMapping {
        field: "title",
        datatype: BiblatexDataType::Literal,
        crate_accessor: Some("entry.title()"),
        target: BiblatexFieldTarget::Mapped("title"),
        note: Some(
            "Read via `rich_field_str`, not the crate's `title()` accessor: converts citeproc-js HTML rich-text markup (e.g. `<span class=\"nocase\">`) to Djot (bean csl26-6eoi).",
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
            "biblatex `publisher` is an `and`-separated literal list (multiple publishers); extraction concatenates it to a single string via `rich_field_str`, discarding list structure.",
        ),
    },
    BiblatexFieldMapping {
        field: "institution",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.institution()"),
        target: BiblatexFieldTarget::Mapped("publisher.name (fallback)"),
        note: Some(
            "Falls back to `organization`, then `school`, when `publisher` is absent. Same list-flattening caveat as `publisher`.",
        ),
    },
    BiblatexFieldMapping {
        field: "organization",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.fields.get(\"organization\") => Vec<Chunks>"),
        target: BiblatexFieldTarget::Mapped("publisher.name (fallback)"),
        note: Some("Same list-flattening caveat as `publisher`."),
    },
    BiblatexFieldMapping {
        field: "school",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.school()"),
        target: BiblatexFieldTarget::Mapped("publisher.name (fallback)"),
        note: Some("Same list-flattening caveat as `publisher`."),
    },
    BiblatexFieldMapping {
        field: "location",
        datatype: BiblatexDataType::LiteralList,
        crate_accessor: Some("entry.location()"),
        target: BiblatexFieldTarget::Mapped("publisher.place"),
        note: Some("Alias of `address`. Same list-flattening caveat as `publisher`."),
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
            "Split on `,` in `biblatex_monograph`; not applied outside the `Monograph` builder.",
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
            "`entry.editors()` also returns `editora`/`editorb`/`editorc` and each field's `EditorType` (`editortype` etc: Compiler, Founder, Continuator, Redactor, Reviser, Collaborator, Organizer, Director); extraction flattens all of it to a single undifferentiated editor list and discards the `EditorType`. See bean csl26-qtur's contributor-roles follow-up.",
        ),
    },
    BiblatexFieldMapping {
        field: "translator",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.translator()"),
        target: BiblatexFieldTarget::Mapped("translator / contributors[translator]"),
        note: None,
    },
    // --- Unmapped: the enumerated gap list (bean csl26-11h2's open items) ---
    BiblatexFieldMapping {
        field: "eprint",
        datatype: BiblatexDataType::Verbatim,
        crate_accessor: Some("entry.eprint()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "No producer of `MonographType::Preprint` reads this. Needs a precedence rule: whether `eprint` on an otherwise-typed entry overrides the entry-type-driven mapping, or only applies to generic/misc entries.",
        ),
    },
    BiblatexFieldMapping {
        field: "eprinttype",
        datatype: BiblatexDataType::Key,
        crate_accessor: Some("entry.eprint_type()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Alias of `archiveprefix`. See `eprint`."),
    },
    BiblatexFieldMapping {
        field: "series",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "§2.2.1 datatype is literal, not entrykey — prior art against modeling a BibLaTeX series as a fully embedded parent `Collection`; a flat `series` field on `Monograph`/`Collection` matches the manual's own model more closely.",
        ),
    },
    BiblatexFieldMapping {
        field: "eventtitle",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "The conference/event name for `@inproceedings`, distinct from `booktitle` (the proceedings volume). No schema slot on `CollectionComponent` today.",
        ),
    },
    BiblatexFieldMapping {
        field: "venue",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Same schema-slot gap as `eventtitle`."),
    },
    BiblatexFieldMapping {
        field: "chapter",
        datatype: BiblatexDataType::Literal,
        crate_accessor: None,
        target: BiblatexFieldTarget::Unmapped,
        note: Some("No schema slot on `CollectionComponent` today."),
    },
    BiblatexFieldMapping {
        field: "afterword",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.afterword()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Editorial sub-role field with its own typed accessor; not read."),
    },
    BiblatexFieldMapping {
        field: "annotator",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.annotator()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Editorial sub-role field with its own typed accessor; not read."),
    },
    BiblatexFieldMapping {
        field: "commentator",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.commentator()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Editorial sub-role field with its own typed accessor; not read."),
    },
    BiblatexFieldMapping {
        field: "foreword",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.foreword()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Editorial sub-role field with its own typed accessor; not read."),
    },
    BiblatexFieldMapping {
        field: "introduction",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.introduction()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some("Editorial sub-role field with its own typed accessor; not read."),
    },
    BiblatexFieldMapping {
        field: "holder",
        datatype: BiblatexDataType::Name,
        crate_accessor: Some("entry.holder()"),
        target: BiblatexFieldTarget::Unmapped,
        note: Some(
            "Patent holder. `Patent` already exists as a standalone reference class in citum-schema-data::reference::types::specialized, unused by BibLaTeX conversion today (see `BIBLATEX_FALLBACK`).",
        ),
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
    /// Which builder converts this entry type: `"monograph"`, `"inbook"`, or
    /// `"article"`.
    pub builder: &'static str,
    /// The resulting `MonographType` wire value, when `builder` is
    /// `"monograph"`.
    pub monograph_type: Option<String>,
    /// Rationale; `None` when the mapping is unremarkable.
    pub note: Option<&'static str>,
}

impl From<&BiblatexEntryMapping> for BiblatexEntryTypeDescriptor {
    fn from(row: &BiblatexEntryMapping) -> Self {
        let monograph_type = match &row.builder {
            BiblatexBuilder::Monograph(mono_type) => Some(mono_type.as_str().to_string()),
            BiblatexBuilder::Inbook | BiblatexBuilder::Article => None,
        };
        Self {
            entry_type: row.entry_type,
            builder: row.builder.name(),
            monograph_type,
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
        let row = biblatex_entry_mapping("patent");

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
