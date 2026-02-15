---
# csl26-gidg
title: Reach 90% oracle match rate
status: todo
type: milestone
priority: critical
created_at: 2026-02-07T12:12:01Z
updated_at: 2026-02-07T12:12:01Z
blocking:
    - csl26-u1in
---

Achieve 90% oracle verification match rate across top 50 parent styles.

**Current status (2026-02-15):**
- Top 10 citations: 9/10 at 15/15 match (90%, Springer regression tracked)
- Top 10 bibliography: varies by format
  - Author-date: 6-14/15 (APA 14/15, Elsevier Harvard 8/15, Chicago 6/15)
  - Numeric: 0/15 (blocked on year positioning feature)
- Coverage: 10/10 converted = 60% dependent corpus (4,792/7,987 styles)

**Target: 90% (top 50 styles)**
Stretch: 95% (top 100 styles)

**Blockers:**
- Numeric style year positioning (affects 6/10 top styles)
- Volume/issue formatting variations
- Springer citation regression

**Focus areas:**
- Author-date bibliography quality refinement (iterate 6/15 -> 12/15+)
- Numeric style features (year positioning, numbering, superscript)
- Systematic failure pattern documentation

**Measurement:**
- Run oracle-batch-aggregate.js weekly
- Track regression from baseline
- Document failure categories in docs/TIER_STATUS.md

Refs: scripts/oracle-batch-aggregate.js, RENDERING_WORKFLOW.md, TIER_STATUS.md