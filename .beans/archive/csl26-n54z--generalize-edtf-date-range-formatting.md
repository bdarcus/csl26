---
# csl26-n54z
title: Generalize EDTF date-range formatting
status: completed
type: feature
priority: high
created_at: 2026-07-26T19:23:00Z
updated_at: 2026-07-26T19:35:56Z
---

Extend docs/specs/EDTF_DATE_RANGE_FORMATTING.md for PR #1103: locale MF2 shared-year patterns, same-era BCE Chicago condensation, tests, schema, and documentation.


## Checklist

- [x] Add locale MF2 shared-year date-range rendering.
- [x] Extend Chicago year-range abbreviation to same-era BCE/CE.
- [x] Update docs, schema, and verification evidence.


## Summary of Changes

Added locale-driven shared-year EDTF interval patterns, same-era BCE/CE Chicago year abbreviation, Spanish MF2 examples, documentation, and regression coverage. Verified schema generation, focused tests, Chicago oracle/workflow batch, smell audit, and the full pre-commit gate.
