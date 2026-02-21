---
# csl26-yrri
title: 'fix(labels): AMS et-al takes 4 initials not 3'
status: todo
type: bug
priority: normal
created_at: 2026-02-21T15:25:34Z
updated_at: 2026-02-21T15:25:34Z
---

In labels.rs, et_al branch hardcodes .take(3) + marker. AMS/citeproc-js takes first 4 initials with no marker for 5+ authors. Fix: make the take count configurable (4 for ams, 3 for alpha/din). Workaround in styles/american-mathematical-society-label.yaml already sets et-al-marker:'' but still emits 3 initials. Tracked in PR #211.
