---
# csl26-4ndr
title: 'Chicago: date detail (month/day) dropped from citations'
status: in-progress
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - dates
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-09-04T13:02:23Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 264 entries where month/day detail present in the oracle (e.g. 'New York Times, March 15') is dropped in Citum's rendering. No prior bean. Likely decomposes into several date-part wiring gaps per type-variant -- treat as a triage bucket, not a single root cause. Touches all four Chicago variants.

## Session scope (2026-09-04): shortnb base-template wave

`chicago-shortened-notes-bibliography-core.yaml`'s base bibliography template date slot renders `date: issued, form: year` only -- no month-day form -- for the fallback path other types (article-newspaper, article-magazine, webpage, post, etc.) drop through to. 125 of 387 shortnb failing rows carry a missing month/day. Fix: apply the same `date: issued, form: month-day` + `, "-prefixed form: year` group pattern already used correctly elsewhere in this file (e.g. its own `standard:` block's patent-number date group, ~line 608-616) to every affected type-variant in one pass, verified per-type against tests/fixtures/test-items-library/chicago-18th.json. Plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md
