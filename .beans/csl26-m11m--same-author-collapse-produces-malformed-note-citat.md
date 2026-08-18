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
updated_at: 2026-08-18T17:49:26Z
parent: csl26-h7oc
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
