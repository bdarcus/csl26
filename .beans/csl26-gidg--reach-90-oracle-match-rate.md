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

Current status: 74% (50 styles, 5/5 citation match)
Target: 90% (top 50 styles)
Stretch: 95% (top 100 styles)

Focus areas:
- Citation rendering fidelity
- Bibliography rendering completion
- Systematic migration failure patterns

Measurement:
- Run oracle-batch-aggregate.js weekly
- Track regression from baseline
- Document failure categories

Refs: scripts/oracle-batch-aggregate.js, RENDERING_WORKFLOW.md