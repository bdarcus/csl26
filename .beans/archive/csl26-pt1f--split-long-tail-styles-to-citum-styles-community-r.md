---
# csl26-pt1f
title: Split long-tail styles to citum-styles community repo
status: completed
type: task
priority: high
tags:
    - architecture
    - styles
created_at: 2026-07-28T14:40:01Z
updated_at: 2026-07-29T19:17:47Z
parent: csl26-s2rw
---

Execute the move-to-community rows of scripts/report-data/style-disposition-2026-07-28.tsv (125 styles). Spec: docs/specs/STYLE_INHERITANCE.md (portfolio policy). Audit: docs/architecture/audits/2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md.

- [x] USER ACTION: create citum-styles GitHub repo
- [x] Move the 125 move-to-community styles there with a README stating provenance, parity status (advisory), and that styles may extends: embedded parents via the registry filesystem layer
- [x] Remove moved files from citum-core; keep 16 keep-exemplar styles and styles/experimental/
- [x] Preserve the alias_review flags (90 styles) in the community repo metadata for later human raw-output review
- [x] Verify cargo nextest run and node scripts/report-core.js still pass
- [x] Update styles/README.md tier description

## Summary of Changes

Seeded `citum-styles` with the 125 relocated styles and filtered disposition metadata, removed the same styles from `citum-core`, and retained the 16 declared exemplars plus experimental styles.
