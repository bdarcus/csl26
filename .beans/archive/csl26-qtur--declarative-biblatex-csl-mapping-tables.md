---
# csl26-qtur
title: Declarative biblatex + CSL mapping tables
status: completed
type: task
priority: normal
tags:
    - docs
    - schema
    - conversion
created_at: 2026-07-27T12:20:10Z
updated_at: 2026-07-27T12:46:41Z
blocking:
    - csl26-11h2
---

Refactor crates/citum-refs biblatex entry-type dispatch and crates/citum-schema-data CSL conversion into declarative consts (BIBLATEX_ENTRY_TYPES, BIBLATEX_FIELDS + BiblatexDataType keyed on BibLaTeX manual sec 2.2.1 data types, CSL_TYPE_MAP promoted from contract_tests EXPECTATIONS), emitted as docs/schemas/type-map.json via citum schema CLI. Strictly behavior-preserving -- no new entry types, no newly-read fields, no extraction rewrite. Prerequisite for the outward-facing data model docs (PR 2) and for csl26-11h2's modeling decisions, which become tractable against the generated gap table.

## Summary of Changes

Refactored the BibLaTeX entry-type dispatch (`crates/citum-refs/src/formats/biblatex/mapping.rs`) and CSL 1.0.2 conversion contract (`crates/citum-schema-data/src/reference/conversion/contract_tests.rs`) from inline matches/test-only tables into declarative, publicly-inspectable consts:

- New `crates/citum-refs/src/formats/biblatex/tables.rs`: `BIBLATEX_ENTRY_TYPES` + `BIBLATEX_FALLBACK` (drives dispatch, behavior-preserving) and `BIBLATEX_FIELDS` keyed on BibLaTeX manual sec 2.2.1 datatypes (declarative only in this PR -- extraction still runs through the existing closures/crate accessors). 46 field rows, including an explicit `Unmapped` gap list covering every item named in csl26-11h2 (eprint, eprinttype, series, eventtitle, venue, chapter, and the six unread contributor name fields), plus gender and the entrykey fields (crossref/xdata/related/relatedtype) as prior art for WorkRelation.
- Promoted `contract_tests::EXPECTATIONS` to `pub const CSL_TYPE_MAP` in `citum-schema-data::reference::conversion::mod`, with each inline rationale comment turned into a structured `note` field. Pure move -- `contract_tests.rs` now asserts against the promoted const with identical assertions.
- `crates/citum-cli`: new `schema type-map` output (and folded into `--out-dir`), emitting `docs/schemas/type-map.json` via `citum_io::biblatex::{biblatex_entry_type_descriptors, biblatex_field_descriptors}` and `citum_schema_data::reference::conversion::CSL_TYPE_MAP` -- no new crate dependencies needed (both already reachable through existing re-export chains).
- `docs/reference/BIBLATEX_MAPPING.md`: added a stale-data notice pointing at `type-map.json` pending the generated replacement (csl26-veeg).

Scope fence held: no new entry types, no newly-read fields, no extraction rewrite, no assertion values changed. All 2214 tests pass (`just pre-commit` green: fmt, clippy -D warnings, nextest). `docs/schemas/type-map.json` rides the existing CI regenerate-and-diff gate (`ci.yml` "Verify JSON schemas are up to date") automatically since it's emitted into the same `--out-dir`.

Confirmed via `type-map.json` inspection that the entry-type table's `collection`/`mvcollection`/`proceedings`/`mvproceedings` rows all resolve to `monograph`/`book` and the fallback row (`*`) resolves to `monograph`/`document` -- matching the "Known corrections" table in the parent plan and giving csl26-11h2 a real gap table to work against.

Unblocks csl26-veeg (data model docs) and gives csl26-11h2 a generated coverage matrix instead of prose.
