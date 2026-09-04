---
# csl26-cip0
title: 'Chicago: DOI-derived URL duplicates a second archive/publisher URL'
status: todo
type: task
priority: normal
tags:
    - style
    - chicago
    - fidelity
created_at: 2026-09-04T13:03:13Z
updated_at: 2026-09-04T13:03:31Z
parent: csl26-h7oc
---

chicago-18-base.yaml has many independent - variable: doi / - variable: url pairs; field-absent: doi gating is inconsistent, so some entries render both a doi.org link and a second jstor/cambridge/oxfordmusiconline URL where the oracle keeps only one. 55 carrying rows, 12 confirmed sole-cause flips. Needs a systematic sweep of every doi/url pair in the base file, not a one-off fix. Found during the 2026-09-04 engine-fix leverage pass, plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md
