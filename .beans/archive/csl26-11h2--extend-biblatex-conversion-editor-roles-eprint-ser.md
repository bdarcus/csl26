---
# csl26-11h2
title: 'Extend BibLaTeX conversion: editor roles, eprint, series, remaining types'
status: completed
type: task
priority: normal
tags:
    - conversion
    - schema
    - fidelity
created_at: 2026-07-24T11:28:35Z
updated_at: 2026-07-27T20:56:28Z
---

Deferred field/entry-type gaps found while investigating csl26-2mse, each genuinely blocked on a decision only Bruce can make (a new enum variant, a modeling choice, or separately-scoped effort) -- not just more of the same mapping work already done in that bean's fix.

- [x] Editor sub-roles (annotator/commentator/foreword/introduction/afterword/holder): done. Five new ContributorRole variants (Annotator, Commentator, ForewordAuthor, IntroductionAuthor, AfterwordAuthor) added and read directly. entry.editors()'s discard of each group's EditorType is fixed -- only EditorType::Editor groups become the editor shorthand; Compiler/Director map to existing roles, Founder/Continuator/Redactor/Reviser/Collaborator/Organizer degrade to ContributorRole::Unknown(name), which round-trips and is selectable by a style as a custom role. data_role_for_custom in citum-engine updated in lockstep to prevent a role-lookup regression (typed variant vs Unknown(str) don't compare equal). holder maps to Patent.assignee (bean's other checklist item).
- [x] eprint/eprinttype -> MonographType::Preprint: done. `EprintInfo` (id/server/class) is now always populated from `eprint`/`eprinttype` (alias `archiveprefix`)/`eprintclass` (alias `primaryclass`) on Monograph/CollectionComponent/SerialComponent. Precedence rule: flips to `MonographType::Preprint` only when `eprint` is present *and* there is no container signal -- a container-less `@article` (no journaltitle/journal), or a misc/unpublished/online/fallback entry. Specifically-typed entries (`@book`, `@thesis`, ...) keep their type even with a stray `eprint`.
- [x] series: done. Reuses the CSL-JSON conversion path's shape for a collection-title (`relation_collection_title`) instead of a new flat field -- an embedded, title-only `Collection` wrapping the series name, wrapped in a title-less parent for `@book`/etc. (no intermediate container-title) or attached directly to the already-synthesized parent Collection/Serial for `@incollection`/`@inproceedings`/`@article`. `number` alongside `series` becomes `NumberingType::Volume` (volume-in-series) rather than a generic document number.
- [x] Remaining entry types: done for patent, dataset, software, standard, periodical, reference/mvreference/inreference. New builder functions per class (build_patent_reference, build_dataset_reference, build_software_reference, build_standard_reference, build_serial_reference), following the build_inbook_reference/build_article_reference pattern. Fixed a real bug along the way: entry-type dispatch collapsed every non-core type (like @standard) to "misc"/"unknown" before it could ever reach the dispatch table, since EntryType::to_string() discards the Unknown(_) payload and .to_biblatex() collapses Unknown to Misc -- dispatch now reads the raw string for Unknown types. map/archive are not real BibLaTeX entry types (they come from other tools) and stay on the generic fallback.
- [x] eventtitle/venue (for @inproceedings), chapter: done. eventtitle/venue/eventdate map onto the synthesized parent Collection's existing `event` field (an embedded Event), the same shape the CSL-JSON paper-conference path already uses -- no schema change needed (the earlier "no schema slot" premise was wrong). Only read for @inproceedings, not @inbook/@incollection. The @inproceedings parent Collection also switched from CollectionType::EditedBook to CollectionType::Proceedings. chapter maps to NumberingType::Chapter on the CollectionComponent itself.
- [x] .bib fixture corpus + contract tests: done. crates/citum-refs/tests/fixtures/biblatex/*.bib (books, parts, serials, reports-theses, specialized, contributors, zotero-shapes -- 7 files, ~17 entries) plus crates/citum-refs/tests/biblatex_conversion.rs, an rstest exercising load_input_refs (the same entry point citum convert refs uses) with exact-match YAML assertions per fixture. Covers series/event/chapter/eprint/collection-proceedings/patent-dataset-software-standard/editor-sub-roles/Zotero nocase-span escaping in realistic multi-field entries, distinct from mapping.rs's single-feature inline unit tests.
- [x] field_str Chunk::Math + literal-list fix: done, narrower than the original framing. `Chunk::Math` is no longer discarded -- wrapped as Djot inline math (`$...$`). `publisher`/`institution`/`organization`/`school`/`location` (all LiteralList datatype) now split on BibLaTeX's `and` separator and rejoin with `"; "` instead of leaking the literal `and`. Did NOT do a generic per-`BiblatexDataType` extraction dispatcher -- only these two narrow fixes were needed, applied directly in `mapping.rs` (`chunk_to_string`/`literal_list_str`). Deliberately left `Date`/`Range`/`Integer` fields on hand-rolled extraction (not the crate's typed accessors), since those normalize input and would change rendered output for already-mapped fields, risking the gb7714 fidelity baseline. Note: `Publisher.name`/`Publisher.place` are still single-valued fields, so a genuine multi-publisher entry still collapses to one string -- only the join delimiter changed, not the underlying schema. Filed as a follow-up if multi-publisher support is ever wanted.

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

## Summary of Changes

All seven checklist items done, across 9 commits on branch `feat/biblatex-conversion-breadth-csl26-11h2`:

1. **Editor roles** -- non-`Editor`-typed `editora`/`editorb`/`editorc` groups no longer collapse into the `editor` shorthand; five new `ContributorRole` variants for the sub-role name fields (annotator/commentator/foreword/introduction/afterword).
2. **eprint** -- `EprintInfo` always populated; `MonographType::Preprint` flip gated on eprint presence + no container signal.
3. **series** -- reuses the CSL-JSON `relation_collection_title` shape instead of a new schema field.
4. **eventtitle/venue/chapter** -- reuse `Collection.event` (the "no schema slot" premise in the original bean text was wrong) and `NumberingType::Chapter`.
5. **Collection/proceedings reclassification** (added mid-investigation, not in the original checklist) -- `@collection`/`@proceedings` move off `Monograph(Book)` onto the `Collection` class.
6. **patent/dataset/software/standard/periodical/reference/inreference** -- five new builders; fixed a real dispatch bug where non-core entry types (like `@standard`) could never reach `BIBLATEX_ENTRY_TYPES` under their own name.
7. **Chunk::Math + literal-list fix** -- narrower than the original "dispatch on BiblatexDataType" framing; math no longer discarded, `and`-separated literal-list fields no longer leak the literal "and".
8. **.bib fixture corpus** -- 7 fixtures, ~17 real entries, exact-match YAML contract tests through `load_input_refs`.

Also split out as its own commit: renamed several inline unit tests off BDD-style (`given_..._when_..._then_...`) naming, which CODING_STANDARDS.md reserves for parameterised integration tests in `tests/`.

Follow-ups filed rather than folded in: csl26-q7zo (locale terms for the 5 new roles) and csl26-0txc (multi-valued Publisher.name/place, deferred -- needs a schema change, only worth it if real corpora show multi-publisher/location entries).

PR not yet opened; CI not yet run.

## PR #1106 Revision\n\nCorrected conference-paper, inreference, and periodical semantic classification; exposed standalone collection series through the accessor; hardened blank patent/standard/eprint handling; completed editorial role localization; corrected generated mapping documentation and fixture expectations; and added semantic regression tests. The representative APA workflow remains at its pre-revision baseline of 20/20 citations and 45/46 bibliography entries.
