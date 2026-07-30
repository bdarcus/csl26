---
# csl26-t6dg
title: Support paired EDTF uncertainty markers for Chicago guessed dates
status: completed
type: bug
priority: normal
tags:
    - dates
    - chicago
    - fidelity
created_at: 2026-07-28T12:18:12Z
updated_at: 2026-07-30T19:37:04Z
parent: csl26-h7oc
blocking:
    - csl26-giun
---

Logical follow-up to `csl26-giun`; parented to its owning Chicago feature because the bean schema does not allow a bug to have a task parent. This bug blocks `csl26-giun`.

Verified conversion evidence: CSL JSON `issued: 1750?` converts to native EDTF `issued: 1750?`; the conversion layer is truthful. Chicago's CSL oracle renders `Smith, John. [1750?]. Title of First Work.`, while Citum renders `Smith, John. 1750? Title of First Work.`. The compatibility report records this as a lenient match and exact-parity failure.

Acceptance criteria:
- [x] Support paired uncertainty markers around an EDTF uncertain year, not only a suffix marker.
- [x] Provide Chicago configuration that renders guessed dates as `[year?]`.
- [x] Add schema, engine, and Chicago regression coverage for uncertain EDTF years.
- [x] Regenerate committed schemas and data-model references required by the schema change.
- [x] Preserve truthful CSL JSON `issued: 1750?` to native EDTF `1750?` conversion.

## Summary of Changes

Added a paired-bracket rendering path for EDTF uncertain years, alongside the existing suffix-only marker:

- New optional `dates.uncertainty-marker-prefix` field on `DateConfig` (`crates/citum-schema-style/src/options/dates.rs`), paired with the existing `uncertainty-marker` (now able to carry the closing bracket, e.g. `?]`). Defaults to `None`, so all existing suffix-only styles (e.g. GB/T 7714's `uncertainty-marker: '?'`) are unaffected.
- `apply_date_markers` (`crates/citum-engine/src/values/date.rs`) now applies the prefix before the formatted year when configured, defaulting to an empty string otherwise.
- `chicago-18-base.yaml` sets `uncertainty-marker: '?]'` + `uncertainty-marker-prefix: '['`, so all four Chicago 18 variants (author-date, notes, shortened-notes-bibliography, T&F) inherit the bracketed `[1750?]` guessed-year rendering.
- Added engine unit tests for the default (suffix-only) and paired-bracket cases, plus schema-level deep-merge/null-clear rstest cases for the new field in `bdd_inheritance.rs`.
- Regenerated `docs/schemas/style.json` via `just schema-gen`.

Verification: `chi-guessed-date` shared-corpus item now renders `Smith, John. [1750?]. Title of First Work.`, an exact match with the citeproc-js oracle (previously a lenient-only match). `node scripts/report-core.js --style chicago-author-date-18th` shows shared-corpus bibliography unaffected elsewhere (339/379, citations 15/15, no regressions vs. the pre-change baseline). Full `just pre-commit` (fmt, clippy, nextest — 2301 tests) green.
