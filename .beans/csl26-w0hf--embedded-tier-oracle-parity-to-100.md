---
# csl26-w0hf
title: Embedded-tier oracle parity to 100%
status: in-progress
type: milestone
priority: high
created_at: 2026-07-30T19:09:35Z
updated_at: 2026-08-02T15:19:04Z
---

Reach 100% divergence-adjusted oracle text parity across the 19 embedded styles while retaining the fidelity and SQI gates. The live report is the measurement source; scripts/report-data/embedded-parity-baseline.json is the checked-in monotonic CI floor; docs/compat.html is the synchronized public snapshot.

Canonical all-features state after the measurement repair: 1415/3249 exact matches (43.6%), with 34 ID-proven not-comparable observations, 26 registered-divergence exclusions, and zero heuristic-unpaired rows. Fresh parallelism-1, fresh parallelism-4, and warm-cache runs are identical. The 1868 residual observations are fully routed to existing beans; Chicago owns 1292 and remains first.

Execution tree:
- csl26-6th8: embedded measurement integrity and residual classification.
- csl26-40n4: Chicago family substrate and completion.
- csl26-ccdt: embedded non-Chicago completion.

Completion requires every eligible row to match or carry an authority-backed exclusion, no unclear adjudications, authoritative ID pairing wherever available, green fidelity/SQI gates, synchronized report artifacts, and completed child/blocker beans.
