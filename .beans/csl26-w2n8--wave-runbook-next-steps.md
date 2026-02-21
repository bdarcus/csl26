---
# csl26-w2n8
title: Wave runbook next steps
status: todo
type: task
priority: high
created_at: 2026-02-21T13:12:33Z
updated_at: 2026-02-21T13:12:33Z
blocking: []
---

Use the canonical wave runbook to execute the next slice:

- Source doc:
  - docs/architecture/MIGRATE_ENHANCE_WAVE_RUNBOOK_2026-02-21.md

Deliverables:
1. Start Wave 3 baseline for the planned 12 styles.
2. Produce baseline oracle metrics and mismatch clusters.
3. Apply migrate/processor promotions only for repeated mismatch patterns.
4. Re-run Wave 3 and capture before/after deltas.
5. Re-run core quality drift checks and record results.

Hard gates:
- Fidelity must not regress on previously closed Wave 1/2 citation behavior.
- For Rust changes, run: cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run.
