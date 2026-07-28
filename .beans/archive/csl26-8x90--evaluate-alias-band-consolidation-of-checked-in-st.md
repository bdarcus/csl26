---
# csl26-8x90
title: Evaluate alias-band consolidation of checked-in styles
status: completed
type: task
priority: normal
tags:
    - taxonomy
    - scorecard
    - styles
created_at: 2026-07-17T17:53:32Z
updated_at: 2026-07-28T14:40:27Z
---

102 of 141 checked-in styles/*.yaml render >=0.98-similar to another registered style on the strict fixture set (scripts/report-data/alias-candidates-band-registered-2026-07-17.tsv). Evaluate which can become registry aliases or thin wrappers instead of full standalone YAML - human review required per pair (fixture-bounded similarity; family near-ties are unstable). Highest-leverage maintenance reduction identified by the 2026-07-17 measurement; pairs with the compat inheritance view (csl26-zik7). Context: docs/architecture/audits/2026-07-17_EXTENDS_DELTA_DERIVABILITY.md

## Summary of Changes

Resolved by disposition rather than alias/wrapper consolidation. Evidence and reasoning: docs/architecture/audits/2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md; per-style table: scripts/report-data/style-disposition-2026-07-28.tsv (141 styles: 16 keep-exemplar, 125 move-to-community, 90 of the moves flagged alias_review for later human raw-output review).

The planned rerun under the refactored exact-parity reporting confirmed the consolidation framing is superseded: the 2026-07-17 addendum found zero safely auto-registrable aliases (normalized similarity too weak), delta re-derivation loses fidelity (1/28 expressible), and exact parity shows the long tail is loosely converted (mostly 0-0.3). Relocation to the citum-styles community repo (csl26-pt1f, epic csl26-s2rw) is the maintenance-reduction lever; aliasing remains a human-gated review queue tracked via the alias_review column. No new 6 GB derivability sweep was required.
