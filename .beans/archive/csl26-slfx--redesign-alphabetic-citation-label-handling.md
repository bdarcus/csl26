---
# csl26-slfx
title: Redesign alphabetic citation-label handling and migrate existing styles
status: completed
type: task
priority: deferred
tags:
    - schema
    - engine
    - citations
created_at: 2026-08-03T18:50:04Z
updated_at: 2026-08-04T16:24:02Z
blocked_by:
    - csl26-49r0
---

Track the follow-up redesign of processor-owned alphabetic `citation-label` handling after numeric citation labels. Make labels declarative while preserving compatibility for existing template-owned labels, and revise those existing styles to use the feature; no new styles are required.

- [ ] Define declarative semantics for citation-label generation and wrapping.
- [ ] Implement schema, migration, renderer, and collapse behavior.
- [ ] Revise existing styles to use the declarative feature and add parity coverage.

## Superseded

PR #1136's declarative-label mechanism, which this bean would extend, is being
redesigned under [[csl26-49r0]] (spec: docs/specs/REFERENCE_MARKERS.md).

An implementation of this bean on the current mechanism was completed and closed
unmerged as PR #1137: CI green, alpha and american-mathematical-society-label
byte-identical to main, no oracle drift on any of eight affected styles. It was
closed because it confirmed the mechanism, not the alphabetic gap, is the
problem. Re-land alphabetic labels and the two style conversions on the new
model as part of csl26-49r0 rather than reviving this branch.

## Resolution

Superseded and delivered by csl26-49r0: declarative alphabetic labels landed as `label-mode: alphabetic`, and `alpha` and `american-mathematical-society-label` were converted to it, both byte-identical to the pre-change baseline.
