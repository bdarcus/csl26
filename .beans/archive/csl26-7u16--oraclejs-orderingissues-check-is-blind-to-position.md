---
# csl26-7u16
title: oracle.js orderingIssues check is blind to positional bibliography divergence
status: completed
type: bug
priority: normal
tags:
    - oracle
    - sorting
    - bibliography
    - testing
created_at: 2026-08-14T14:17:53Z
updated_at: 2026-08-14T18:16:53Z
---

scripts/oracle.js's orderingIssues check pairs bibliography entries by id, not by position, so
it cannot detect that Citum renders two entries in a different order than citeproc even when
every individual entry's rendered text matches exactly (component-level and even exact-string
match per entry). This let a real bug through undetected: chicago-author-date-18th's
bibliography sort ordered "Johnson, Brian" before "Johnson, Alice" (citeproc: Alice first) and
"Smith, John" before "Smith, Jane" (citeproc: Jane first) — orderingIssues reported 0 in both
the buggy and fixed state.

Root cause of the underlying bug (fixed in csl26-tc4x): sort_support::structured_name_sort_text
ignored given names when building the bibliography author sort key, so family-name ties fell
through to title order instead of breaking on given name.

## Todo
- [x] Change orderingIssues (or add a new check) to compare oracle vs citum bibliography
      entry order positionally, not just per-id text match
- [x] Re-run the full corpus with the new check to see how many other styles have undetected
      ordering divergence

## Summary of Changes

**Detection gap closed** — `scripts/lib/oracle-divergences.js` gained `compareBibliographyOrder`,
an unconditional positional comparison of the oracle vs. Citum bibliography entry-id sequence,
run regardless of whether any individual entry failed (the old gate only ran order inspection
when `citations.failed || bibliography.failed > 0`, exactly the condition the Johnson/Smith bug
violated: zero failures, wrong order). `attachRegisteredDivergenceAdjustments` now returns
`bibliographyOrder: { oracleOrderIds, citumOrderIds, firstDivergentIndex, appliedDivergence,
explained }`, with `explained` computed from the *residual* sequence after setting aside
div-004/div-008-attributed ids — not just "a rule fired somewhere" (an early version of this
was unsound: it would have permanently masked an unrelated reorder in any style that also had
an anonymous item). Wired through `oracle.js`, `oracle-fast.js`, and `report-core.js`
(`bibliographyOrderMismatch` per style); `check-core-quality.js` warns (not fails) on an
unexplained mismatch.

**Real engine bug found and fixed while verifying** — the corpus sweep immediately surfaced
17/35 core styles with a real order mismatch. Tracing the first case (chicago-author-date-18th)
led to `crates/citum-engine/src/sort_support.rs`: `flat_names_sort_key` and
`contributor_sort_key`'s `ContributorList` branch only ever compared the *first* author's
family+given, then fell through to title — even for multi-author references. citeproc-js
compares every co-author in the list before falling back (verified directly against
`CSL.Engine` with a minimal style). Fixed both functions to walk the full contributor list.
Two existing integration tests encoded the old (wrong) tie behavior and were updated with their
expected order corrected — again verified against `CSL.Engine` directly for each exact shape,
not by inference — the same discipline `structured_name_sort_text`'s fix (csl26-tc4x) used.
Added unit + integration regression coverage for the new full-list comparison, each confirmed
to fail without the fix and pass with it (not just insertion-order or id-tiebreak coincidences).

**div-008 was not retired.** Investigated whether it was dead post-tc4x (the premise for
retiring it); the corpus sweep falsified that — it still explains 6/35 styles even after this
session's engine fix (down from 9/35 pre-fix). `verification-policy.yaml` /
`DIVERGENCE_REGISTER.md` are untouched.

**8/35 core styles still show an unexplained order mismatch** after the engine fix — triaged
(not fixed) in follow-up bean `csl26-rmjp`, including a confirmed citum-migrate gap
(elsevier-harvard's legacy CSL declares a descending-year secondary sort key that the migrated
YAML dropped entirely).

Verification: `node --test scripts/*.test.js scripts/lib/*.test.js` (291/291),
`cargo nextest run --no-fail-fast` (2533/2533), `cargo fmt --check` + `cargo clippy --all-targets
--all-features -- -D warnings` clean, `check-core-quality.js` and `check-oracle-regression.js`
green against a fresh sequential `report-core.js --all-features` sweep.

## Post-review corrections (Copilot findings on PR #1186)

Three real defects found by Copilot's review, fixed by amending the two commits above:

1. **`flat_names_sort_key`'s literal-first-name special case discarded
   subsequent co-authors** — a mixed contributor list (institutional first
   author + personal co-authors) tied completely and fell through to title,
   reproducing the exact bug this PR fixes, just for organizational-first
   lists. The per-name loop already handled literals correctly; removed the
   redundant early return so the whole list is always walked.
2. **The `explained` residual check was unsound** — deleting div-008's
   affected ids from both sequences before comparing residuals discards
   those ids' position relative to the rest of the sequence, so an
   unrelated id moving *around* the explained cluster could still look
   "explained." Replaced with `canonicalizeAffectedIdsToOracleOrder`, which
   corrects only the specific cluster in place (same slots, oracle's
   relative order) and requires the *whole* sequence to then match.
3. **`mergeOracleResults` never combined `bibliographyOrder`** across
   family-fixture-set/benchmark-run merges, so an unexplained mismatch
   found only in a merged-in extra fixture was silently dropped. Added
   `mergeBibliographyOrderSignals`, prioritizing an unexplained mismatch
   over an explained or absent one.

**Corrected corpus sweep** (fix #2 was strictly more conservative — it
un-masks cases the old logic wrongly called explained):

| | Original PR | After review fixes |
|---|---|---|
| Styles with an order mismatch | 17/35 | 17/35 (unchanged) |
| Explained by div-004/div-008 | 9 | 4 |
| Unexplained | 8 | 13 |
| div-008 still genuinely explaining anything | 6 styles | 1 style (`harvard-cite-them-right`) |

`check-core-quality.js` and `check-oracle-regression.js` both still green
against the corrected sweep (diagnostic-only warning, as designed — 14
warnings now vs. 4 before, still non-fatal). Follow-up bean `csl26-rmjp`
updated with the corrected unexplained list.
