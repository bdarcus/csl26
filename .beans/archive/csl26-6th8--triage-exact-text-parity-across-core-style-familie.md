---
# csl26-6th8
title: Embedded parity measurement integrity and residual classification
status: completed
type: epic
priority: high
tags:
    - scorecard
    - styles
    - fidelity
created_at: 2026-07-28T12:17:54Z
updated_at: 2026-08-02T15:16:28Z
parent: csl26-w0hf
---

Coordinate trustworthy measurement and bounded root-cause classification for the 19 embedded styles only. Earlier work delivered the exact-parity gate and adjudication ledger; the remaining portfolio-wide 157-style scope is retired for this milestone.

Order of work:
- Repair the canonical report and synchronized artifacts through child beans csl26-pf22, csl26-5okt, csl26-7xhp, and csl26-waly.
- Generate one authoritative embedded report after those repairs.
- Assign every remaining embedded mismatch to a bounded implementation bean or an explicit adjudication state.
- Record affected-row counts and affected embedded styles in each implementation bean so ready work can be ranked by impact and cost.

Acceptance criteria:
- [ ] Canonical all-features reporting succeeds without profiler artifacts.
- [ ] ID-bearing bibliography output replaces heuristic pairing wherever IDs are available.
- [ ] Parallelism 1 and 4 produce identical per-style exact-parity totals.
- [ ] Baseline JSON and docs/compat.html are regenerated from the same successful commit.
- [ ] Every embedded residual is assigned to a bounded bean or explicit adjudication state.
- [ ] No generic unclassified bucket remains.

 Classification completed 2026-08-02 from the canonical all-features report at fcfc7407. Scope: 19 embedded styles; 1,834 eligible exact-parity failures plus 34 ID-proven one-sided bibliography rows = 1,868 residual observations. Mechanical clustering produced 921 stable signatures and assigned 100% of observations to existing bounded beans, with no needs-bean, needs-adjudication, or unclear states. Observation routing: csl26-giun 752; csl26-7jht 476; csl26-lxy3 49; csl26-t0m4 14; csl26-ucg3 1; csl26-ww77 480; csl26-6eak 26; csl26-l5oh 47; csl26-y49d 14; csl26-lhrl 4; csl26-m8la 3; csl26-3az5 2. Chicago therefore remains first at 1,292/1,868 current residual observations. Validated working artifact: /tmp/csl26-6th8-clusters.json (19 per-style records, assignedObservations === totalResidualObservations, unassignedObservations 0).
