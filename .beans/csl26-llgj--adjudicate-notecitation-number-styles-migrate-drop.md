---
# csl26-llgj
title: Adjudicate note+citation-number styles migrate drops
status: todo
type: task
priority: low
tags:
    - migrate
    - citation
    - note-styles
created_at: 2026-08-19T13:48:15Z
updated_at: 2026-08-19T13:48:22Z
---

4 real `class="note"` corpus styles (e.g. `proinflow.csl`) declare `collapse="citation-number"`, but stripping the attribute and re-rendering shows it produces same-author-style author-suppression merging (`GARCIA, Maria. …; GARCIA, Maria. …` → `GARCIA, Maria. …; …`), not numeric range compression — citeproc-js's citation-number collapse mechanics evidently work differently for full-sentence note styles than the Citum `CitationNumber`/`SameAuthor` model captures.

Migrate currently drops the attribute for these 4 styles (maps to *absent*, same as no `collapse` at all) rather than emitting a value that would fail the new regime-coherence validation (`same-author` is illegal on `Numeric`; `citation-number` is illegal on `Note`). This is a known, deliberately out-of-scope gap from `docs/specs/SAME_AUTHOR_COLLAPSE.md` §6's caveat and §4's acceptance criteria.

## Scope
- [ ] Determine whether these 4 styles should migrate to `same-author` instead of having the attribute dropped entirely, or whether a third collapse mechanism is needed.
- [ ] If `same-author` is the right target, add `Note`-regime handling and re-run `extract_citation_collapse` mapping.
- [ ] Re-run `report-core.js` for the affected styles to confirm no regression from any migrate change.

See `docs/specs/SAME_AUTHOR_COLLAPSE.md` §6, §7 note (embedded styles), and the corpus measurement table.
