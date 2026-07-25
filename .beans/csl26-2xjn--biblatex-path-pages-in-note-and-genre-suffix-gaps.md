---
# csl26-2xjn
title: 'Biblatex path: pages-in-note and genre-suffix gaps vs CSL-JSON'
status: todo
type: bug
priority: normal
tags:
    - fidelity
    - rendering
    - biblatex
created_at: 2026-07-25T14:06:54Z
updated_at: 2026-07-25T14:07:01Z
---

Follow-up from csl26-7ab8. After fixing biblatex contributor/editor/translator/pages serialization, the gb7714-bench corpus (typst-doc-cn/bib-csl-dev-data, GB-T_7714—2025.builtin) 344-entry .bib-vs-.json bibliography diff dropped from 301/344 differing to 206/344. Residual causes are unrelated to contributors:

1. Zotero's BibLaTeX export stuffs page ranges into a freeform `note = {Pages: N}` field rather than the standard `pages` field for many entries (~30 of the 206); the CSL-JSON source has a proper `page` field. The biblatex converter has no way to know `note` sometimes encodes pages without parsing that convention specifically.
2. The large majority of the remaining diffs (~176) are missing GB/T type-suffix tags ([M]/[J]/[S]/[P]/[DS]/[A]/[N]) and other genre-derived formatting -- likely a genre/type mapping gap between biblatex's coarser entry-type vocabulary and CSL-JSON's richer `type` field, independent of this bean's contributor fix.
3. At least one patent entry (gbt7714.8.10.2:1) renders with missing punctuation/field separators entirely, a distinct formatting defect.

Re-run the corpus diff (fetch data/GB-T_7714—2025.builtin.{bib,json} from typst-doc-cn/bib-csl-dev-data via `gh api`, convert both through `citum convert refs`, render both through `citum render refs -s gb-t-7714-2025-numeric -m bib -j`, strip leading [N], diff by id) to confirm scope before starting.

## Todo
- [ ] Root-cause the pages-in-note convention and decide whether/how to parse it
- [ ] Root-cause the genre/type-suffix mapping gap between biblatex and CSL-JSON paths
- [ ] Fix the patent entry punctuation defect
- [ ] Re-run the corpus diff and confirm the count drops further
