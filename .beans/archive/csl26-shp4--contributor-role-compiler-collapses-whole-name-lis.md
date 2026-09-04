---
# csl26-shp4
title: Contributor role compiler collapses whole name list (maps to Composer not Compiler)
status: completed
type: bug
priority: high
tags:
    - style
    - chicago
    - fidelity
    - contributors
created_at: 2026-09-04T13:02:52Z
updated_at: 2026-09-04T17:19:05Z
parent: csl26-h7oc
---

## Summary of Changes

Implemented directly rather than via a separate spec-first PR — on
review the change is a fixed-size, mechanical extension (one enum
variant + one compiler-enforced-adjacent match arm + a few YAML config
lines), not the kind of open design decision the project's spec-first
convention is meant to gate. docs/specs/COMPILER_CONTRIBUTOR_ROLE.md's
PR (#1260) was closed without merging; its investigation is preserved
here.

Changes:
1. Added `Compiler` to `citum_schema::template::ContributorRole`
   (crates/citum-schema-style/src/template.rs).
2. Extended `contributor_role_to_reference_role`
   (crates/citum-engine/src/values/contributor/mod.rs) — this match has
   a trailing `_ => None`, so it is NOT compiler-enforced despite
   looking exhaustive; verified by hand and by testing, not by relying
   on a compile error.
3. Extended `data_role_for_builtin` in
   crates/citum-engine/src/values/contributor/substitute.rs — a second,
   separate match that the primary-substitution `candidates` path
   actually calls through (`resolve_candidate` ->
   `contributor_for_candidate` -> `lookup_role_contributor` ->
   `data_role_for_builtin`); missed in the original investigation,
   found by testing end-to-end rather than trusting the plan.
4. Fixed `parse_role_name`
   (crates/citum-schema-style/src/locale/raw_conversion.rs) mapping
   "compiler" to Composer instead of Compiler.
5. Added `compiler` as a substitute candidate: chicago-author-date-18th
   (citation AND bibliography scope — inherited by
   taylor-and-francis-chicago-author-date), and
   chicago-shortened-notes-bibliography's bibliography scope only.

Scope note: shortnb's CITATION scope could not be fixed here. Its
citation template gates the contributor+title group on
`render-when: field-present/absent: author` (a check against the raw
`author` field, not the substitution-resolved effective contributor) —
the same pre-existing gap already tracked as csl26-x79y. Confirmed by
testing: a citation-scope substitute config for compiler had zero
observable effect until reverted (kept out per the project's
no-speculative-config principle). Fixing csl26-x79y properly would
unblock this and any other authorless-reference citation case for
notes-style Chicago, not just compiler.

YAML syntax note: `options.substitute.candidates` entries for a
contributor role need the map form `- contributor: compiler`, not a
bare `- compiler` string — only the five `SubstituteField` values
(collection-editor/editor/parent-serial/title/translator) parse as bare
scalars.

Verified against the two real fixtures (6188419/CRTE2HQ7 "Austin, Tim,
comp.", 6188419/Q9GCH7RF "Gray, Rosemary, comp.") via CLI render, plus
a new integration test
(compiler_candidate_promotes_to_primary_slot_when_author_absent in
crates/citum-engine/tests/cross_role_contributors.rs) and a unit test
for the locale-term fix. Full-portfolio per-entry diff (35 embedded
styles): +3 exact-parity rows (chicago-author-date-18th 219/542,
taylor-and-francis-chicago-author-date 219/542,
chicago-shortened-notes-bibliography 92/473), zero regressions.
Regenerated embedded-parity-baseline.json, the report-core.test.js
pinned count, and the shortnb coverage-audit manifest.

Checked whether PRIMARY_CONTRIBUTOR_SUBSTITUTION.md, 
ROLE_SUBSTITUTE_FALLBACK.md, or CROSS_ROLE_CONTRIBUTOR_LISTS.md needed
updating for this change: no. All three are written generically over
whatever `ContributorRole` variants exist; none enumerates specific
role names or would need to special-case compiler.
