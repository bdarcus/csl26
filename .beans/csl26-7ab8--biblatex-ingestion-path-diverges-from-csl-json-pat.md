---
# csl26-7ab8
title: Biblatex ingestion path diverges from CSL-JSON path on 301/344 benchmark entries
status: todo
type: bug
priority: high
tags:
    - biblatex
    - fidelity
    - rendering
created_at: 2026-07-25T12:18:08Z
updated_at: 2026-07-25T12:18:34Z
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
