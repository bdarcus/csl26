---
# csl26-20l7
title: Refocus compat report headline on embedded set
status: todo
type: task
priority: normal
tags:
    - scorecard
    - styles
created_at: 2026-07-28T14:40:02Z
updated_at: 2026-07-28T14:40:19Z
parent: csl26-s2rw
blocked_by:
    - csl26-pt1f
---

After the community split, scripts/report-core.js default selection = embedded styles (headline metrics incl. exact parity) + keep-exemplar styles as a labeled secondary tier; community styles drop out of the default report. Spec: docs/specs/STYLE_INHERITANCE.md. Use a worktree for clean baseline diffs (report-core baseline workflow).

- [ ] Default style selection: embedded + exemplar tiers, tier labels in JSON and compat.html
- [ ] Headline portfolio metrics computed over embedded tier only
- [ ] Refresh docs/compat.html and core-quality baseline
- [ ] Keep --styles-dir escape hatch for community-repo runs
