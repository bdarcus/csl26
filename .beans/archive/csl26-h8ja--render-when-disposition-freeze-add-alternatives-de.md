---
# csl26-h8ja
title: 'render-when disposition: freeze, add alternatives:, defer work-form routing'
status: completed
type: task
priority: high
tags:
    - schema
    - fidelity
    - style
    - engine
    - architecture
created_at: 2026-09-06T21:53:19Z
updated_at: 2026-09-06T21:59:29Z
---

Decision record + spec work resolving whether to extend/remove/freeze render-when. See plan at /home/bruce/.claude/plans/do-another-style-improvement-purring-eclipse.md.

## Todo
- [x] Write decision record under docs/architecture/ (PM/domain-expert audience, worked examples first)
- [x] Draft docs/specs/ALTERNATIVES.md
- [x] Amend docs/specs/RENDER_WHEN_CONTRACT.md to v1.2 (freeze vocabulary)
- [x] Draft docs/specs/MEDIUM_DESIGNATOR.md ([Internet] marker, independent of render-when decision)
- [x] Cross-link csl26-x79y, csl26-x61x
- [x] File work-form-routing design bean under csl26-40n4 (csl26-zmxt)
- [x] Update csl26-zs9y root cause

## Summary of Changes

Produced the docs-only decision record and specs for render-when disposition, per the plan at /home/bruce/.claude/plans/do-another-style-improvement-purring-eclipse.md.

- docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md: full-corpus inventory (125 uses, 123 in Chicago), A/B/C/D shape classification, per-field breakdown showing the top 6 fields (108/125 uses) are each a mix of fallback (A) and structural-policy (B) shapes, so no single replacement retires render-when cleanly.
- docs/specs/ALTERNATIVES.md (Draft v1.0): ordered-candidate-list primitive for the A-shape (fallback) uses, generalizing Substitute.candidates, DateFallbackCandidate, and ArticleJournalNoPageFallback. Fixes csl26-x79y by construction (tests output, not raw field presence).
- docs/specs/RENDER_WHEN_CONTRACT.md v1.2: field vocabulary frozen, no further extension; points new needs at alternatives: or the deferred work-form-routing design.
- docs/specs/MEDIUM_DESIGNATOR.md (Draft v1.0): new bibliography option for the [Internet]/Available-from/[cited] bundle in the vancouver/NLM family, the one wave-3 need alternatives: does not cover, confirmed against the shipped .csl macros.
- csl26-zs9y updated with the reclassified root cause; csl26-x79y and csl26-x61x cross-linked; csl26-zmxt filed under csl26-40n4 for the deferred work-form-routing design (the 25 B-shape uses with no replacement yet).

Style wave-3 parity fixes (PR 1 in the plan) not started this session; the session was explicitly reoriented onto the render-when question per user request. Next: user review of the decision record and the two Draft specs, then implementation in a stacked PR; separately, the wave-3 style-only fixes remain available any time.
