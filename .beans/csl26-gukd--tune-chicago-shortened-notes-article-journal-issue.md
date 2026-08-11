---
# csl26-gukd
title: Tune Chicago shortened-notes article-journal issue/year grammar
status: in-progress
type: task
priority: high
created_at: 2026-08-11T15:03:58Z
updated_at: 2026-08-11T17:25:07Z
parent: csl26-h7oc
---

Bounded style-evolve tune pass for chicago-shortened-notes-bibliography.

Target cluster: article-journal bibliography issue/year grammar identified by the current registered coverage audit.

## Acceptance Criteria

- [x] Article-journal issue/year output matches the authority cluster without regressing fidelity.
- [x] Exact parity does not regress and the shared-ancestor portfolio floor is checked.
- [x] Coverage packet is regenerated current and style QA approves.
- [x] Final stacked PR #1168 is opened above PR #1162.

\nEvidence: article-journal exact parity improved 34/473 → 48/473; fidelity held at 0.681; SQI remained 1. The registered packet stayed current after regeneration, with rendered 151 → 157 and uncovered 200 → 194 observations; joined audit parity remained 28/80 because the packet is structural/leaf-scoped evidence.



Final pass evidence: explicit Chicago title casing across inherited bibliography variants plus legal-case and article-magazine variants raised exact parity 48/473 → 60/473, fidelity 0.681 → 0.691, bibliography fidelity count 295/440 → 300/440, and SQI remained 1. Full-portfolio report confirmed no sibling embedded-style floor regressions.
