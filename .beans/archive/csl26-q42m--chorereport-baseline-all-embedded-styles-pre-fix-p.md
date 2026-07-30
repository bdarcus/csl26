---
# csl26-q42m
title: 'chore(report): baseline all embedded styles pre-fix + parity ratchet'
status: completed
type: task
priority: high
created_at: 2026-07-30T14:18:11Z
updated_at: 2026-07-30T14:21:07Z
parent: csl26-arly
---

scripts/report-data/core-quality-baseline.json currently only records ieee and springer-vancouver-brackets (2 of 19 embedded styles). check-core-quality.js's hard fidelity<1.0 gate is therefore vacuous for the other 17. Record all 19 at current pre-fix values (including a new non-gating exactParity field per style), confirm check-core-quality.js tolerates the new field, then re-ratchet post-fix at the end of this PR. Part of PR-1 (fix/embedded-parity-wave-1).

## Summary of Changes

Recorded a non-gating oracle-text-parity snapshot for all 19 embedded styles at HEAD (828cb9d2, matches the branch commit — verified before writing, since the source report used for taxonomy analysis was stamped at fb0a01af and intervening commits included style YAML changes) in scripts/report-data/embedded-parity-baseline.json.

**Correction from the original plan:** the plan called for adding all 19 styles to scripts/report-data/core-quality-baseline.json. That file's `styles` map is read by scripts/check-core-quality.js as a hard fidelityScore==1.0 gate keyed by membership -- adding any style below 1.0 fidelity (17 of 19 embedded styles today) would have immediately broken that gate for the whole embedded tier. core-quality-baseline.json is correctly scoped as a ratchet for styles that have *already* reached 1.0 fidelity (today: ieee, springer-vancouver-brackets only) and is left untouched. Sub-1.0 ratchets for benchmark-run pass rates already exist per-style in scripts/report-data/verification-policy.yaml (min_pass_rate, actively used by the Chicago-tuning epic csl26-40n4) -- but nothing recorded oracle text parity (exactParity) anywhere. That's the actual gap this closes: a snapshot for future parity-improvement waves to diff against. Confirmed no other script reads core-quality-baseline.json besides check-core-quality.js and its test (check-testing-infra.js/.test.js) and the fidelity.yml CI workflow -- the new file is fully independent, no collision risk.
