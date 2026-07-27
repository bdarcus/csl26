---
# csl26-11h2
title: 'Extend BibLaTeX conversion: editor roles, eprint, series, remaining types'
status: in-progress
type: task
priority: normal
tags:
    - conversion
    - schema
    - fidelity
created_at: 2026-07-24T11:28:35Z
updated_at: 2026-07-27T17:06:50Z
---

Deferred field/entry-type gaps found while investigating csl26-2mse, each genuinely blocked on a decision only Bruce can make (a new enum variant, a modeling choice, or separately-scoped effort) -- not just more of the same mapping work already done in that bean's fix.

- [ ] Editor sub-roles (annotator/commentator/foreword/introduction/afterword/holder): entry.editors() in citum-refs/src/formats/biblatex/mapping.rs flattens all editorial roles into one undifferentiated `editor` field. ContributorRole enum (crates/citum-schema-data/src/reference/contributor.rs:197-213) has no variants for these -- needs new variants added first. Separately, even the *existing* editor field already loses information before it gets this far: mapping.rs's `e.into_iter().flat_map(|(persons, _)| persons).collect()` (around line 222) discards each editor's `EditorType` (Compiler/Founder/Continuator/Redactor/Reviser/Collaborator/Organizer/Director) while flattening editora/editorb/editorc into one list -- fix that discard regardless of whether/when the six unread name fields above get their own ContributorRole variants. Check whether unknown roles already degrade acceptably via ContributorRole's `tolerant_enum!` before adding new variants.
- [x] eprint/eprinttype -> MonographType::Preprint: done. `EprintInfo` (id/server/class) is now always populated from `eprint`/`eprinttype` (alias `archiveprefix`)/`eprintclass` (alias `primaryclass`) on Monograph/CollectionComponent/SerialComponent. Precedence rule: flips to `MonographType::Preprint` only when `eprint` is present *and* there is no container signal -- a container-less `@article` (no journaltitle/journal), or a misc/unpublished/online/fallback entry. Specifically-typed entries (`@book`, `@thesis`, ...) keep their type even with a stray `eprint`.
- [x] series: done. Reuses the CSL-JSON conversion path's shape for a collection-title (`relation_collection_title`) instead of a new flat field -- an embedded, title-only `Collection` wrapping the series name, wrapped in a title-less parent for `@book`/etc. (no intermediate container-title) or attached directly to the already-synthesized parent Collection/Serial for `@incollection`/`@inproceedings`/`@article`. `number` alongside `series` becomes `NumberingType::Volume` (volume-in-series) rather than a generic document number.
- [ ] Remaining entry types with no obvious InputReference target: patent, dataset, software, standard, map, archive, periodical, reference/mvreference/inreference. Note: citum-schema-data/src/reference/types/specialized.rs already defines standalone Patent/Dataset/Standard/Software reference classes (not MonographType variants) -- mapping to these would follow the same pattern as build_inbook_reference/build_article_reference (a new builder function per class), not a schema change. Real, multi-struct implementation work, worth its own pass rather than folding into a single-field fix.
- [x] eventtitle/venue (for @inproceedings), chapter: done. eventtitle/venue/eventdate map onto the synthesized parent Collection's existing `event` field (an embedded Event), the same shape the CSL-JSON paper-conference path already uses -- no schema change needed (the earlier "no schema slot" premise was wrong). Only read for @inproceedings, not @inbook/@incollection. The @inproceedings parent Collection also switched from CollectionType::EditedBook to CollectionType::Proceedings. chapter maps to NumberingType::Chapter on the CollectionComponent itself.
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
