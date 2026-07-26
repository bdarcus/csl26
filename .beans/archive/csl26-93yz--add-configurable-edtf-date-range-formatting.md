---
# csl26-93yz
title: Add configurable EDTF date-range formatting
status: completed
type: feature
priority: high
created_at: 2026-07-26T18:16:38Z
updated_at: 2026-07-26T18:35:27Z
---

Implement docs/specs/EDTF_DATE_RANGE_FORMATTING.md: date-local expanded/chicago year-range formatting; enable Chicago 18; tests and generated schema.



## Checklist

- [x] Add the date-range configuration and shared Chicago formatter helper.
- [x] Configure Chicago 18 and add an active feature spec.
- [x] Regenerate schema and validate the implementation.
- [x] Complete the feature bean with verification results.



## Summary of Changes

Added date-local EDTF year-range formatting with expanded and Chicago modes, enabled Chicago 18 condensation, regenerated the schema, and verified with targeted tests, the Chicago workflow test, and `just pre-commit`.
