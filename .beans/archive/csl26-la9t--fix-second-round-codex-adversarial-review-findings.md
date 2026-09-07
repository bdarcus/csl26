---
# csl26-la9t
title: Fix second-round Codex adversarial-review findings
status: completed
type: task
priority: high
tags:
    - schema
    - fidelity
    - style
    - engine
    - architecture
created_at: 2026-09-07T00:18:59Z
updated_at: 2026-09-07T00:49:40Z
---

Second Codex adversarial review of docs/render-when-alternatives-decision (after the first fix pass, csl26-8b4a) found 4 more findings (2 high, 2 medium). Plan and verification at /home/bruce/.claude/plans/do-another-style-improvement-purring-eclipse.md.

## Todo
- [x] RENDER_WHEN_CONTRACT.md: add 6 missing TemplateConditionField table rows
- [x] MEDIUM_DESIGNATOR.md: add cited_date_label, distinguish NLM/springer (term.cited) from CSE (term.accessed)
- [x] MEDIUM_DESIGNATOR.md: add per-style exact-output fixture acceptance criterion
- [x] Decision record: fix stale NLM-DOI bullet in main Recommendation section
- [x] ALTERNATIVES.md: rewrite tracker rule to state nested-group dependency on csl26-2hr4
- [x] Elevate csl26-2hr4 to blocking prerequisite for alternatives:
- [x] Re-run Codex adversarial review a third time to confirm resolution -- re-ran; round-2 findings resolved, but surfaced 4 new/deeper issues, tracked in csl26-6vqz

## Summary of Changes

Fixed all 4 round-2 Codex findings (commit 404a4258): incomplete frozen render-when table, wrong accessed-date term for CSE, stale contradictory NLM-DOI text in the decision record, insufficient tracker isolation for nested groups. Re-running the review to confirm surfaced 4 more, deeper findings (2 high, 2 medium) -- tracked separately in csl26-ro72.
