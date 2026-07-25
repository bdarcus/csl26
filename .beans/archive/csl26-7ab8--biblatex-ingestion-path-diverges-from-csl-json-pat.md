---
# csl26-7ab8
title: Biblatex ingestion path diverges from CSL-JSON path on 301/344 benchmark entries
status: completed
type: bug
priority: high
tags:
    - biblatex
    - fidelity
    - rendering
created_at: 2026-07-25T12:18:08Z
updated_at: 2026-07-25T14:07:14Z
---

Discovered while fixing csl26-6eoi (raw citeproc HTML leaking into rendered output).
Comparing citum's rendered output for the same 344-entry gb7714-bench corpus
(typst-doc-cn/bib-csl-dev-data) via the CSL-JSON path vs the biblatex path shows
301/344 entries differ beyond the HTML-leak fix -- the biblatex path drops authors
and page ranges wholesale:

  gbt7714.5.1:1
    json: 博伯尔. 银行业的未来与人工智能[M]. 徐超，译. 北京：清华大学出版社，2023：35.
    bib :        银行业的未来与人工智能[M].          北京：清华大学出版社，2023.

This is a far larger benchmark fidelity gap than the nocase leak (builtin.bib rendered
via `citum convert refs` then `citum render`). Needs its own root-cause investigation --
likely author/contributor extraction and page-range extraction gaps in
crates/citum-refs/src/biblatex.rs (`input_reference_from_biblatex`,
`build_inbook_reference`, `build_article_reference`, `biblatex_monograph`).

## Repro
1. Fetch typst-doc-cn/bib-csl-dev-data's GB-T_7714—2025.builtin.json and
   GB-T_7714—2025.builtin.bib
2. Render both through gb-t-7714-2025-numeric --mode bib --json
3. Diff entries by id (strip leading `[N]` numbering first) -- 301/344 differ

## Summary of Changes

Root cause: the biblatex converter builds `InputReference`s directly in Rust rather than through deserialization, so the `author`/`editor`/`translator` shorthand fields it filled were `skip_serializing` and never folded into the canonical `contributors` vec — the only field serialization preserves. CSL-JSON and RIS were unaffected because both route through legacy conversion, which populates `contributors` explicitly. This was serialization-only: rendering straight from a `.bib` worked because accessors fall back to the shorthand, which is why every existing in-memory test passed.

Fixed in two commits (fix/biblatex-contributor-serialization branch):
- `fix(schema)`: added `normalize_contributors` on the five structural reference types and on `InputReference`, refactored the five `From<...Deser>` impls to use it (single reconciliation path for both deserialization and direct construction).
- `fix`: called it from `input_reference_from_biblatex`; mapped comma-less biblatex names to `SimpleName` literals instead of `StructuredName` with an empty `given`; unwrapped single-contributor lists to match the CSL-JSON/RIS shape; fixed `entry.editors()` being treated as present even when empty (leaked a bogus `contributor: []`); moved `@article` editor onto the parent `Serial` and `@inbook`/`@incollection` translator onto the component only (both driven by which accessor has a container fallback); mapped `pages` on `Monograph`.

Added 6 new tests in `crates/citum-refs/src/biblatex.rs` asserting on serialized YAML (not just the in-memory reference, which was the whole reason this bug survived), plus a round-trip idempotency test. All 2193 workspace tests pass; `cargo fmt --check`, clippy (`-D warnings`), and `just schema-gen` (no schema diff) all clean.

Corpus re-diff (gb7714-bench, GB-T_7714—2025.builtin, 344 entries, .bib-vs-.json via `citum convert refs` + `citum render refs -s gb-t-7714-2025-numeric -m bib -j`): differing entries dropped from 301/344 to 206/344. The exact repro from this bean's description (`gbt7714.5.1:1`) now matches on author/translator; the residual 206 are unrelated causes (pages embedded in freeform `note` fields, genre/type-suffix mapping gaps, a patent-entry punctuation defect) — tracked separately in csl26-2xjn.
