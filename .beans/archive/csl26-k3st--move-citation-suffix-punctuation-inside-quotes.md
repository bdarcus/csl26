---
# csl26-k3st
title: Move citation suffix punctuation inside quotes
status: completed
type: bug
priority: high
tags:
    - rendering
    - punctuation
    - engine
created_at: 2026-08-09T18:04:30Z
updated_at: 2026-08-09T18:35:32Z
---

Citation-level suffix punctuation is appended by `Processor::apply_spec_wrap_and_affixes`
after the assembled component string, so an enabled punctuation-in-quote policy cannot move
a terminal period or comma inside a closing quoted title. The shortened-notes baseline exposes
this on `single-item-1`, `single-item-3`, `single-item-4`, and related observations. Follow-up
to csl26-1hya.

- [x] Route applicable citation-spec period and comma suffixes through the locale-aware quote boundary.
- [x] Add exact tests for enabled and disabled punctuation-in-quote placement.
- [x] Compare fresh-cache parent and child shortened-notes reports.
- [x] Confirm the exact-parity denominator and not-comparable count do not change.
- [x] Run the authoritative Rust pre-commit gate (2,447 tests passed).

Measured result: exact parity rises from 20/473 to 34/473 (+14 exact outputs), with the
473-output denominator and 16 not-comparable outputs unchanged. Twenty citation strings change
only at their terminal quote/period boundary; no bibliography output changes.
