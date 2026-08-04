---
# csl26-49r0
title: Redesign processor-owned reference markers across citation regimes
status: completed
type: feature
priority: normal
tags:
    - schema
    - engine
    - citations
created_at: 2026-08-04T11:16:36Z
updated_at: 2026-08-04T16:23:53Z
---

Give processor-generated reference markers (`[1]`, `[Kuh62]`) a first-class representation in the engine render model, and remove the label component from the template surface so no AST node can denote a marker. Author-date primary keys are template-composed and are not markers; note anchors are an output-format contract.

Spec: docs/specs/REFERENCE_MARKERS.md

Supersedes csl26-slfx. PR #1137 implemented declarative alphabetic labels on the existing mechanism and was closed unmerged: it worked (CI green, byte-identical style output, no oracle drift) but confirmed the mechanism is the problem. See the PR for the evidence.

`number: citation-number` and `number: citation-label` are removed from the schema (breaking). Twelve shipped styles author one today and are converted in the same change; three of them (`ieee`, `american-medical-association`, `numeric-comp`) already declare `label-mode` as well, which is the redundancy this closes.

- [x] Move spec status Draft -> Active in the first implementation commit.
- [x] Route marker resolution through one module (`processor/rendering/marker.rs`), typed plan. Output-neutral.
- [x] Add `citation.options.item-wrap` (three wrap scopes) and convert `ieee` to it.
- [x] Convert the remaining styles to declarative markers.
- [x] Reject authored markers at parse time; migrate emits label-mode.
- [ ] Remove the two `NumberVariable` variants outright. Blocked on rendering the marker outside the template pipeline instead of synthesizing a node; see the spec Acceptance criteria.
- [x] Add `bibliography.options.label-separator` as the marker-slot separator.
- [x] Re-land declarative alphabetic labels on the new model.
- [x] Hold report-core fidelity and exact-parity green (36 styles fidelity 1.0, exact-parity >= baseline for 19 embedded-core styles).

Remaining internal work (marker still materialized as a template node) is tracked on [[csl26-j54l]].

## Summary of Changes

Reference markers are now processor-owned end to end.

- `processor/rendering/marker.rs` is the single owner of marker resolution; eleven helpers that were spread across the rendering module are private to it.
- `citation-number` / `citation-label` are rejected at parse time. Declaring `label-mode` is the only way to ask for a marker.
- Added `citation.options.item-wrap` (the third wrap scope) and `bibliography.options.label-separator` (the marker-slot gap).
- Alphabetic markers gained `label-mode: alphabetic`; a declared mode strips a marker of any other kind inherited from a parent.
- All shipped styles declare their marker. Two incomplete migrations fixed against their CSL sources: royal-society-of-chemistry now marks every entry (its CSL renders citation-number in the bibliography layout), gb-t-7714-2025-author-date marks none (its CSL renders it nowhere).
- citum-migrate emits `label-mode` and no marker component.

Verified: `just pre-commit` green at every commit; `just check-core-quality` passed (36 styles fidelity 1.0, exact-parity >= baseline for 19 embedded-core styles); byte-diff against a main baseline worktree across 20 styles x (bibliography + citations) clean apart from the two intentional fidelity fixes.

Remaining internal work — the marker is still materialized as a template node, so the enum variants survive — is tracked on [[csl26-j54l]].
