---
# csl26-4xr6
title: 'Chicago: title case not applied to bibliography titles'
status: completed
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - title
created_at: 2026-08-23T20:40:25Z
updated_at: 2026-08-23T22:08:59Z
parent: csl26-h7oc
---

Leverage class from docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md wave 1. 182 entries where a type-variant simply omits the title-case transform (YAML wiring), e.g. 'Mesopotamia: between two rivers' vs oracle 'Mesopotamia: Between Two Rivers'. Bundles the sibling over-capitalization sub-cause (31 entries: real stop-word gaps against citeproc-js's skipWordsRex -- in/into/via -- plus post/article-newspaper headlines that should stay sentence case). Single largest measured defect in the family, no prior bean. Do NOT touch docs/policies/TEXT_CASE_PROTECTION.md's internal-caps preservation mechanism (csl26-4kt3) while fixing this -- the 2-entry acronym/mixed-case sub-class (PhD->Phd) is adjacent but out of scope here. Touches all four Chicago variants. Use node scripts/analyze-parity-residuals.js to re-measure.

## Summary of Changes

Narrower and more targeted than the audit's estimate turned out to be
achievable in one bounded pass -- landed with zero regressions,
verified against the full 2,691-test Rust suite and per-entry
exactMatch diffing (not just aggregate counts).

- chicago-author-date-18th.yaml: added `map`/`dataset` to the
  existing `titles.type-mapping` (extends the established pattern
  for broadcast/collection/manuscript/motion-picture/song/webpage).
  Measured delta: 0 -- title-case is now correct on these entries, but
  they remain compound-failing on the quote-boundary defect (class B,
  csl26-jxco), which the oracle also requires. Not a null result: it's
  evidence the two classes are more entangled here than a flat count
  suggested, and a prerequisite for wave 2 to succeed on these entries.
- chicago-notes-18th.yaml + chicago-shortened-notes-bibliography-core.yaml:
  converted `titles: chicago` (a shared preset with no text-case, also
  used by MLA) into an explicit inline block adding `text-case: title`
  to component/monograph/container-monograph, mirroring
  chicago-author-date-18th's own explicit config. Measured delta: +2
  and +5 respectively.
- Deliberately did NOT add broadcast/collection/manuscript/motion-picture/
  song/webpage to these two styles' type-mapping: a first attempt did,
  and it regressed 3 previously-passing archival/manuscript entries by
  pulling bare collection titles ("Revere Family Papers", "Landscapes
  of Zambia, Central Africa") into the same `quote: true` that
  correctly applies to genuine article titles. Reverted to the narrower
  map/dataset-only scope; the rest is deferred to wave 2 (csl26-jxco)
  for per-type verification.
- taylor-and-francis-chicago-author-date-core.yaml: deliberately left
  untouched. It explicitly nulls the inherited type-mapping (Style F
  sentence-cases article titles; the null prevents other types from
  leaking into that bucket -- a documented, deliberate design decision,
  not an oversight). Extending it risked colliding with that intent.

Net: chicago-author-date-18th 191/542 (unchanged), taylor-and-francis
191/542 (unchanged), chicago-notes-18th 28/72 -> 30/72 (+2),
chicago-shortened-notes-bibliography 81/473 -> 86/473 (+5). Family
491/1,629 -> 498/1,629 (+7). Zero regressions (verified per-entry, not
just aggregate); cargo nextest run: 2691/2691 passed.

Validates the 2026-08-23 leverage-audit method on real style work: the
projected 304-row wave-1 gain does not materialize 1:1 because A1
(title-case) and B (quote boundary) are more entangled than a flat
per-class count implies -- itself a useful, now-documented finding for
wave 2's scope.
