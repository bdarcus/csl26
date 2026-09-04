---
# csl26-xj32
title: Locale-aware smart-quote conversion for literal-authored ASCII quotes
status: todo
type: feature
priority: low
tags:
    - style
    - engine
    - rendering
created_at: 2026-09-04T13:03:31Z
updated_at: 2026-09-04T13:03:31Z
---

Some CMOS-18 fixture entries author literal ASCII straight quotes/apostrophes in titles or free-text fields; the oracle (citeproc-js) round-trips them through typographic curly quotes, citum currently does not. This is a new engine feature (locale-aware smart-quote normalization at render time), not a bug fix -- 31 carrying rows, 3 confirmed sole-cause flips, low volume. Found during the 2026-09-04 Chicago engine-fix leverage pass; do not build opportunistically inside an unrelated PR stack. Plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md
