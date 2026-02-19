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

**Current status (2026-02-19, strict oracle):**
- Top 10 citations: 0/10 at 8/8 match
- Top 10 bibliography: 1/10 at 100% (APA 27/27), others 0-26/34
- Coverage: 10/10 converted = 60% dependent corpus (4,792/7,987 styles)
- Citation fixture expanded to include suppress-author and mixed locator/prefix/suffix cases

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
- Use `tests/fixtures/citations-expanded.json` as canonical citation scenario set
- Track regression from baseline
- Document failure categories in docs/TIER_STATUS.md

Refs: scripts/oracle-batch-aggregate.js, RENDERING_WORKFLOW.md, TIER_STATUS.md
