---
# csl26-wolw
title: Refresh compat.html How-To-Read section for portfolio
status: completed
type: task
priority: normal
created_at: 2026-07-30T13:05:02Z
updated_at: 2026-07-30T13:10:48Z
parent: csl26-s2rw
---

PR D of style-corpus split cleanup. Update generateHtmlSqiExplainer() in scripts/report-core.js to explain the three-tier portfolio (embedded/exemplar/community), the family header rows in generateHtmlTable(), and that biblatex is a separate authority from citeproc-js. Verify the compatibility and SQI targets still hold post-refocus before restating them. Regenerate docs/compat.html via node scripts/report-core.js --write-html; refresh styles/README.md and docs/reference/SQI.md if targets change. Closes csl26-20l7 and csl26-zi01. Depends on PR A (csl26-5lt5, archived/completed). Full plan: /home/bruce/.claude/plans/there-are-some-style-imperative-wand.md section D

## Summary of Changes

Made generateHtmlSqiExplainer() data-driven (takes report, computes live
embedded-tier mean compatibility/SQI) instead of static text. Added: the
three-tier portfolio explanation (embedded/exemplar/community, with current
counts), an explanation of the family header rows generateHtmlTable() emits
(root = top of the extends chain; most styles are still singleton families),
and a note that biblatex is a separate authority from citeproc-js, naming
numeric-comp as the current example.

Verified the stated ">=95% compatibility / >=90 SQI" targets: they were
never enforced as literal numbers anywhere in code (the real gate,
check-core-quality.js, checks per-style baseline drift, not an aggregate).
Current live measurement is 94.7% mean compatibility / 96.9 mean SQI across
the 19 embedded styles -- kept as directional working targets, but the
explainer now shows the current measured value alongside the target instead
of only the stale target, and docs/reference/SQI.md was updated to match
(mean-across-embedded-tier framing, explicit pointer to the real gate).

docs/compat.html regenerated via --write-html; 216/216 JS tests pass.
