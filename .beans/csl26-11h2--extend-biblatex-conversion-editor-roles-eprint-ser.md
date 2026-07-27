---
# csl26-11h2
title: 'Extend BibLaTeX conversion: editor roles, eprint, series, remaining types'
status: todo
type: task
priority: normal
tags:
    - conversion
    - schema
    - fidelity
created_at: 2026-07-24T11:28:35Z
updated_at: 2026-07-27T14:24:10Z
---

Deferred field/entry-type gaps found while investigating csl26-2mse, each genuinely blocked on a decision only Bruce can make (a new enum variant, a modeling choice, or separately-scoped effort) -- not just more of the same mapping work already done in that bean's fix.

- [ ] Editor sub-roles (annotator/commentator/foreword/introduction/afterword/holder): entry.editors() in citum-refs/src/formats/biblatex/mapping.rs flattens all editorial roles into one undifferentiated `editor` field. ContributorRole enum (crates/citum-schema-data/src/reference/contributor.rs:197-213) has no variants for these -- needs new variants added first. Separately, even the *existing* editor field already loses information before it gets this far: mapping.rs's `e.into_iter().flat_map(|(persons, _)| persons).collect()` (around line 222) discards each editor's `EditorType` (Compiler/Founder/Continuator/Redactor/Reviser/Collaborator/Organizer/Director) while flattening editora/editorb/editorc into one list -- fix that discard regardless of whether/when the six unread name fields above get their own ContributorRole variants. Check whether unknown roles already degrade acceptably via ContributorRole's `tolerant_enum!` before adding new variants.
- [ ] eprint/eprinttype -> MonographType::Preprint: MonographType::Preprint exists but nothing produces it. Needs a precedence rule: does an eprint field on an otherwise-typed entry (e.g. @article with eprint) override the entry-type-driven mapping, or only apply to generic/misc entries?
- [ ] series: no flat field on Monograph/Collection; only maps through `container: WorkRelation`, which would mean modeling a BibLaTeX series as a fully embedded parent Collection -- a real modeling decision, not a one-line field read.
- [ ] Remaining entry types with no obvious InputReference target: patent, dataset, software, standard, map, archive, periodical, reference/mvreference/inreference. Note: citum-schema-data/src/reference/types/specialized.rs already defines standalone Patent/Dataset/Standard/Software reference classes (not MonographType variants) -- mapping to these would follow the same pattern as build_inbook_reference/build_article_reference (a new builder function per class), not a schema change. Real, multi-struct implementation work, worth its own pass rather than folding into a single-field fix.
- [ ] eventtitle/venue (for @inproceedings), chapter: no obvious schema slot on CollectionComponent today -- needs a schema-field decision.
- [ ] Build a .bib fixture corpus + BibLaTeX conversion contract tests: no .bib fixtures exist anywhere in citum-core; a native-construction corpus (per this repo's test-coverage conventions) would let the gb7714-bench-derived exact-match-vs-Zotero gap be tracked and regression-tested locally instead of relying on an external, unpersisted CI artifact.
- [ ] Refactor field_str (citum-refs/src/formats/biblatex/mapping.rs) to use biblatex's typed field accessors instead of hand-rolled Chunk-to-string, which currently silently discards Chunk::Math content to an empty string. Touches every field extraction in the file -- a separate robustness concern from mapping breadth, not bundled here. Frame this as dispatch on `BiblatexDataType` (crates/citum-refs/src/formats/biblatex/tables.rs, added in bean csl26-qtur) rather than one-off typed accessors per field -- the datatype is already declared per-field in `BIBLATEX_FIELDS`, so extraction can switch on it generically. This would also fix literal-list flattening of `publisher`/`location` (both are BibLaTeX `and`-separated literal lists today concatenated to a single string via `rich_field_str`, discarding list structure) since both share the datatype-driven fix.

See csl26-2mse's Summary of Changes for what was already fixed (entry-type mapping for techreport/thesis/online/unpublished/proceedings, translator, institution/organization/school publisher fallback, subtitle, abstract/version/keywords, ISBN propagation to synthesized parent Collection).

## Cross-reference: bean csl26-veeg (generated docs)

bean csl26-veeg generated `docs/reference/BIBLATEX_MAPPING.md` from the same
tables.rs/type-map.json this bean touches -- its "Not Yet Mapped" section is
now the authoritative, CI-drift-checked version of this bean's field-gap list
(eprint, eprinttype, series, eventtitle, venue, chapter, and the six editorial
name fields). No new beans were filed for the other csl26-veeg follow-ups
(RIS mapping doc, TYPE_SYSTEM_ARCHITECTURE.md draft status) -- those are
docs-only and stay as prose in docs/reference/DATA_MODEL.md's Follow-ups
section rather than separate tracker entries, to avoid PR-per-checklist-item
proliferation.
