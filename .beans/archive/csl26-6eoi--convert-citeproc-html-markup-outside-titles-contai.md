---
# csl26-6eoi
title: Convert citeproc HTML markup outside titles (container-title, biblatex fields)
status: completed
type: bug
priority: high
tags:
    - nocase
    - csl-json
    - biblatex
    - rendering
    - engine
created_at: 2026-07-25T11:36:00Z
updated_at: 2026-07-25T12:18:34Z
---

Fix raw citeproc-js HTML (<span class=\"nocase\">, <i>, <b>, <sc>, <sup>, <sub>) leaking
into rendered bibliography output. Surfaced by gb7714-bench:
https://gb7714.zhtyp.art/entry/gbt7714.8.6.1-5/

Bean csl26-zaqk fixed this for `title`/`short_title` only (via html_markup_to_djot in
crates/citum-schema-data/src/reference/conversion/mod.rs), explicitly deferring
container-title/publisher/note/etc. This bean is that deferred scope, plus a second,
independently-discovered leak on the biblatex ingestion path (crates/citum-refs/src/biblatex.rs),
which has no HTML->Djot conversion at all.

Baseline (gb-t-7714-2025-numeric, 344-entry corpus from typst-doc-cn/bib-csl-dev-data):
- builtin.json: 2 entries leak raw HTML (container-title)
- better.json: 2 entries leak raw HTML (container-title)
- builtin.bib: 9 entries leak raw HTML (title AND container-title -- biblatex path has zero conversion)
- better.bib: 0 (uses {{...}} brace protection already handled)

Target: 0/0/0/0.

## Plan
- [x] Move html_markup_to_djot + DjotTag + classify_open_tag out of the legacy-convert-gated
      conversion/mod.rs into a new ungated crates/citum-schema-data/src/reference/citeproc_markup.rs
- [x] CSL-JSON path: apply html_markup_to_djot over an explicit allowlist of rich-text fields
      (title, container_title, collection_title, original_title, publisher, publisher_place,
      event, genre, medium, section, authority, abstract_text, archive, archive_location,
      dimensions) in InputReference::from(LegacyReference), after parse_note_field_hacks().
      NOTE: `number` was dropped from the allowlist after verification -- see below.
- [x] Keep the existing html_markup_to_djot calls in build_title (idempotent; needed for
      part-title which comes from the extra map, skipped by the central pass) -- add comment
- [x] Biblatex path: add rich_field_str closure + BibRefContext field in citum-refs/src/biblatex.rs;
      route title/subtitle/publisher/institution/organization/school/location/booktitle/series/
      journaltitle/journal through it
- [x] Tests: moved unit tests for citeproc_markup.rs; conversion-boundary rstest for
      container-title/collection-title; biblatex rstest for escaped-HTML booktitle/title;
      end-to-end regression test for the benchmark's exact gbt7714.8.6.1:5 entry
- [x] Verify: full 344-entry corpus (builtin.json/better.json/builtin.bib/better.bib) renders
      zero raw HTML through gb-t-7714-2025-numeric
- [x] Verify: report-core.js --style gb-t-7714-2025-numeric fidelity unchanged pre/post fix (see Summary -- baseline is actually 0.996/193-203, not zaqk's stale 0.989)
- [x] just pre-commit green (fmt, clippy -D warnings, nextest)
- [x] just schema-gen (citum-schema-data matches citum-schema*) -- expect no diff
- [ ] PR via gh pr create, --body-file, citing benchmark URL and leak-count table

## Deferred (follow-up beans, not in this PR)
- Biblatex path diverges from CSL-JSON path on 301/344 entries (drops authors, page ranges
  wholesale) -- much larger benchmark gap than the nocase leak, needs its own investigation
- Name.literal (institutional authors) as a likely next HTML carrier -- none in this corpus
- RIS ingestion path (citum-refs/src/formats/ris.rs) has the same exposure, unexercised by
  this benchmark
- StringOrNumber fields (volume, issue, edition) could carry the same ordinal markup as
  `number` but no fixture exercises it

## Summary of Changes

Root cause: bean csl26-zaqk converted citeproc-js's literal HTML rich-text convention (`<span class="nocase">`, `<i>`, `<b>`, `<sc>`, `<sup>`, `<sub>`) to Djot at ingestion, but wired the conversion in only at `build_title` -- `title`/`short_title`. Every other free-text CSL-JSON field, and the entire biblatex ingestion path, still leaked raw HTML verbatim into rendered output.

### Changes
- Promoted `html_markup_to_djot` (+ `DjotTag`/`classify_open_tag`) out of the `legacy-convert`-gated `conversion/` submodule into a new ungated `crates/citum-schema-data/src/reference/citeproc_markup.rs`, since the biblatex path needs it too and has nothing to do with CSL-legacy.
- CSL-JSON path: new `normalize_rich_text_markup` runs once in `InputReference::from(LegacyReference)`, right after `parse_note_field_hacks()`, converting an explicit allowlist of 15 fields (title, container_title, collection_title, original_title, publisher, publisher_place, event, genre, medium, section, authority, abstract_text, archive, archive_location, dimensions).
- Biblatex path (`citum-refs/src/biblatex.rs`): added a `rich_field_str` closure alongside the existing `field_str`, routed through `title`/`subtitle`, `publisher`/`institution`/`organization`/`school`/`location`, `booktitle`, `journaltitle`/`journal`, `abstract`, and `type` (genre).
- 11 new/moved tests across `citeproc_markup.rs`, `conversion/mod.rs` (conversion-boundary tests for container-title and collection-title, plus a direct test of `normalize_rich_text_markup` confirming `note` stays untouched), `citum-refs/src/biblatex.rs` (escaped-HTML `title`/`booktitle` cases), and a new end-to-end regression test `crates/citum-engine/tests/gb7714_bench_regression.rs` rendering the exact benchmark entry through the embedded `gb-t-7714-2025-numeric` style.

### `number` field: added then reverted
Initially added `number` to the CSL-JSON allowlist, citing `tests/csl-test-suite/processor-tests/machines/flipflop_NumericField.json` (`"number": "1<sup>er</sup>"`) as evidence. Running that exact fixture through the real `citeproc` npm package (not just eyeballing citum's own render) showed this was backwards: real citeproc-js does NOT flip-flop the `number` variable -- it renders the tag literally, HTML-escaped (`1&#60;sup&#62;er&#60;/sup&#62;`). `number` is a CSL number-type variable, exempt from flip-flop, unlike ordinary text variables. Removed it from the allowlist before committing. Separately verified via the same method (real `citeproc` engine, hyphenated CSL-JSON keys) that every field actually kept in the allowlist -- including `container-title`, `collection-title`, and `publisher-place`, which don't render standalone in citeproc-js's default variable wrapping and needed a slightly different harness to confirm -- DOES flip-flop correctly.

### Verification
- Full 344-entry corpus from all four benchmark sources (typst-doc-cn/bib-csl-dev-data's builtin.json/better.json/builtin.bib/better.bib) rendered through `gb-t-7714-2025-numeric`: 0 entries with raw HTML in any of the four, down from the pre-fix baseline of 2/2/9/0.
- `just pre-commit` green: fmt, clippy -D warnings, 2187 nextest tests.
- `just schema-gen`: no diff (as expected -- no schema-visible type changes).
- report-core.js --style gb-t-7714-2025-numeric: fidelityScore 0.996, GB/T upstream corpus 193/203 passed, IDENTICAL before and after this fix (confirmed by stashing and re-running). Note: this is the actual current baseline -- zaqk's recorded 0.989 had already drifted upward from unrelated commits, unrelated to this work.
- IMPORTANT CAVEAT discovered during verification: the oracle/report-core fidelity pipeline's `normalizeText()` (scripts/oracle-utils.js) strips ALL HTML tags (`.replace(/<[^>]+>/g, '')`) before comparing citum's output to real citeproc-js -- so it is structurally blind to this exact class of bug (raw HTML leaking into output). The identical 193/203 pre/post is a "no regression" signal only, not evidence this specific fix is correct. The real evidence for correctness is (a) the corpus grep for raw HTML tags (0/0/0/0), (b) the conversion-boundary and end-to-end unit/integration tests asserting the exact Djot-converted string, and (c) the direct real-citeproc-js field-by-field flip-flop verification above. Worth fixing separately: `normalizeText`'s HTML-stripping means the oracle can never catch a regression of this exact bug in the future -- filed as a follow-up bean.

## Follow-ups (new beans to create)
- Biblatex path diverges from CSL-JSON path on 301/344 entries in the benchmark corpus (drops authors and page ranges wholesale) -- much larger benchmark gap than this nocase leak; needs its own investigation.
- `Name.literal` on institutional authors as a likely next HTML carrier -- none in this corpus, but plausible in real Zotero data.
- RIS ingestion path (`citum-refs/src/formats/ris.rs`) has the same exposure, unexercised by this benchmark.
- `page`, `volume`, `issue`, `edition` (`StringOrNumber`) -- verified via real citeproc-js that these DO flip-flop, but no fixture in this repo carries rich text there; not added speculatively.
- The oracle's `normalizeText()` strips HTML tags before comparison, making it structurally blind to raw-HTML-leak regressions in general (not just this one). Consider a separate raw-HTML-tag assertion in the fidelity pipeline.
- Pre-existing, not introduced here: `html_markup_to_djot`'s `classify_open_tag` has a false-positive risk -- `a < b > c` would misclassify `< b >` as a `<b>` (Strong) open tag, since it splits on whitespace and `"b"` is a recognized tag name. Only reachable in fields with both `<`/`>` and single-letter tag-name-like text; more plausible now that `abstract_text`/`dimensions` are in the allowlist than when only `title` was.
