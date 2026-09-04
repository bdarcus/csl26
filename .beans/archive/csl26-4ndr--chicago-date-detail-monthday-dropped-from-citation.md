---
# csl26-4ndr
title: 'Chicago: date detail (month/day) dropped from citations'
status: completed
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - dates
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-09-04T15:35:00Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 264 entries where month/day detail present in the oracle (e.g. 'New York Times, March 15') is dropped in Citum's rendering. No prior bean. Likely decomposes into several date-part wiring gaps per type-variant -- treat as a triage bucket, not a single root cause. Touches all four Chicago variants.

## Session scope (2026-09-04): shortnb base-template wave

`chicago-shortened-notes-bibliography-core.yaml`'s base bibliography template date slot renders `date: issued, form: year` only -- no month-day form -- for the fallback path other types (article-newspaper, article-magazine, webpage, post, etc.) drop through to. 125 of 387 shortnb failing rows carry a missing month/day. Fix: apply the same `date: issued, form: month-day` + `, "-prefixed form: year` group pattern already used correctly elsewhere in this file (e.g. its own `standard:` block's patent-number date group, ~line 608-616) to every affected type-variant in one pass, verified per-type against tests/fixtures/test-items-library/chicago-18th.json. Plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md

## Summary of Changes

Fresh measurement (post-PR1/PR4): 102 carrying / 26 sole-cause for
chicago-shortened-notes-bibliography (the plan's stale 125/387 figure
was pre-PR1/PR4). Root cause verified structurally, not assumed: the
style's bibliography section lives entirely in
styles/embedded/chicago-shortened-notes-bibliography-core.yaml (its
parent, chicago-notes-18th.yaml, is notes-only and defines no
bibliography section at all — its own type-variants: block is
citation-scope, a red herring on first read). Two sites used
`date: issued, form: year` where oracle needs the full date: the base
`bibliography.template`'s fallback date slot (used by every
type-variant without its own override — article-newspaper, webpage,
interview, post, broadcast, speech, software, document, etc.) and
`article-magazine`'s own type-variant. Fixed by switching both to
`form: full`, which already handles the year/month-year/month-day-year
punctuation distinction correctly in engine code (crates/citum-engine/src/values/date.rs,
DateForm::Full) — no group/prefix hackery needed. Verified against
oracle for full dates, month-only dates (no comma: "May 2000"), and
date ranges. Full-portfolio per-entry diff: +4 exact-parity rows
(87->91/473), zero regressions across all 35 embedded styles, D
date-detail label-instance count dropped 102->35 (67 rows shed the
defect even where entangled with title-case/quote-boundary/other
classes prevented a full flip). Regenerated
embedded-parity-baseline.json and the report-core.test.js pinned
count.
