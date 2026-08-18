---
# csl26-m11m
title: Same-author collapse produces malformed note citations
status: todo
type: bug
priority: normal
tags:
    - note-styles
    - engine
    - rendering
    - citation
    - chicago
created_at: 2026-08-18T12:49:06Z
updated_at: 2026-08-18T19:15:15Z
parent: csl26-h7oc
blocked_by:
    - csl26-ecfn
---

Found while probing fixture coverage for csl26-uctc (locator-aware collapse
delimiter). Same-author collapse in note styles (chicago-notes-18th,
chicago-shortened-notes-bibliography-core) produces malformed full notes,
independent of locators.

chicago-notes-18th, [@ITEM-31; @ITEM-32]  (Garcia, two different articles,
same author, no locators):

    Garcia, "Methods for Robust Climate Attribution", "Methods for
    Probabilistic Climate Attribution".

The second item's date, journal, and DOI are all dropped -- author-stripping
collapse assumes a year-group to collapse into, but a full Chicago note has
no such group; it's an entire multi-clause sentence per citation. With a
locator it degrades further (date/journal missing, locator misplaced):

    Maria Garcia, "Methods for Robust Climate Attribution," Annual Review of
    Climate Science 4 (2019): 55-80, https://doi.org/..., "Methods for
    Probabilistic Climate Attribution," Annual Review of Climate Science 4
    (2019), 257, https://doi.org/...

This is not the same bug as csl26-uctc (which is about delimiter choice once
collapsed) -- the note-style output is structurally broken before delimiter
choice even matters. Full notes for repeat citations by the same author in
one cluster arguably should NOT collapse the way author-date collapses years
-- CMOS notes typically render each citation close to in full or use ibid/
short forms per position, not a merged sentence. Needs design work, not a
one-line fix.

Blocks: extending tests/fixtures/citations-humanities-note.json or any other
note fixture with same-author multi-item clusters, since there's nothing
correct to pin an assert_eq! to yet.

## Resolved via csl26-ecfn (2026-08-18)

Root cause identified: same-author collapse was never actually note-specific.
It's an unconditional engine behavior that bypasses the existing, genuinely
opt-in `citation.collapse` field entirely — the same defect `csl26-ecfn`
found on `taylor-and-francis-council-of-science-editors-author-date` (an
author-date style with no `collapse` attribute, wrongly collapsing anyway).

citeproc-js ground truth on `chicago-notes-bibliography.csl` (ITEM-31 +
ITEM-32, both Garcia) confirms notes never collapse — full author repeated on
every cite, joined by `"; "`. Two adjacent Citum outputs already match this
byte-for-byte (`note-disambiguate-add-names-et-al`, and a different-author
cluster), proving only the collapse *gate* was wrong, not the per-item
rendering path.

Fix: `docs/specs/SAME_AUTHOR_COLLAPSE.md` makes same-author collapse gate on
`citation.collapse: same-author`. No note-specific code — note styles simply
never declare the setting, so they fall out of the collapse path
automatically. This bean is now blocked-by `csl26-ecfn`, which carries the
implementation. See `csl26-ecfn` for corpus measurements and the field-shape
rationale.

Also flagged, not filed: an independent, pre-existing locator-punctuation
defect (`4 (2019), 257` vs oracle `4 (2019): 257`) noticed while reproducing
this — reproduces on single-item citations, unrelated to collapse, deliberately
out of scope here.
