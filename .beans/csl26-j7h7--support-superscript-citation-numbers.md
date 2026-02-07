---
# csl26-j7h7
title: Support superscript citation numbers
status: todo
type: feature
priority: high
created_at: 2026-02-07T06:53:04Z
updated_at: 2026-02-07T07:40:14Z
blocking:
    - csl26-6whe
    - csl26-l2hg
---

Nature and Cell styles use superscript numbers, not [1] or (Author Year).

Current: (Kuhn 1962)
Expected: ¹

Fix:
- Detect citation-number variable in CSL citation layout
- Detect vertical-align='sup' on number text
- Set citation.template to number-only for numeric styles
- Handle superscript as rendering option in csln_core
- Test against Nature, Cell styles

Refs: GitHub #128, TIER3_PLAN.md Issue 1.1