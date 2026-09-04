---
# csl26-emnp
title: Adjudicate Chicago citeproc-quirk rows (Merriam-Webster.Com, dangling See .)
status: todo
type: task
priority: low
tags:
    - style
    - chicago
    - adjudication
created_at: 2026-09-04T13:03:38Z
updated_at: 2026-09-04T13:03:44Z
parent: csl26-h7oc
---

A handful of Chicago failing rows look citum-correct against a citeproc-js quirk or fixture artifact, same shape as the 2026-08-23 audit's genre-slug adjudication finding: Merriam-Webster.Com Dictionary (oracle title-cases the domain, citum lowercases .com correctly) and CSL needs a way to indicate... See . (oracle has a dangling space before a period from an empty cross-reference target, citum omits the artifact). Candidates for scripts/report-data/parity-adjudication.json, not for a fix. Note: that ledger currently has no consumer in report-core.js/check-core-quality.js (per the 2026-08-23 audit finding #4) so writing entries here won't move the gate yet. Found during the 2026-09-04 Chicago engine-fix leverage pass. Plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md
