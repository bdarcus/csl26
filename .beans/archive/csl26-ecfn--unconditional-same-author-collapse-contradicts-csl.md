---
# csl26-ecfn
title: Unconditional same-author collapse contradicts CSL's opt-in semantics
status: completed
type: bug
priority: normal
tags:
    - rendering
    - disambiguation
    - citation
    - divergence
created_at: 2026-08-16T00:16:16Z
updated_at: 2026-08-19T13:49:09Z
---

Found while validating csl26-p7a8's title-quote flip for
taylor-and-francis-council-of-science-editors-author-date via
report-core.js exactParity comparison against citeproc-js.

The real CSL (taylor-and-francis-council-of-science-editors-author-date.csl)
has no `collapse` attribute at all on <citation>. In CSL 1.0, collapse
is opt-in -- with no attribute, same-author multi-item citations stay
as separate grouped entries joined by the citation's own delimiter.
citeproc-js confirms this:

- disambiguate-year-suffix: oracle "(Garcia 2019a; Garcia 2019b)"
- subsequent-author-consecutive: oracle "(Chen 2022; Chen 2024)"

Citum instead unconditionally collapses same-author consecutive
cites into a comma-joined year list ("(Garcia 2019a, 2019b)",
"(Chen 2022, 2024)"), regardless of the style's `collapse` setting.

This is not a bug in the sense of unintended behavior -- it's
csl26-dpep ("fix(engine): same-author multicite collapse --
first-class rule for both modes"), a deliberate Citum design
decision to always collapse same-author multicites. But that
decision was never recorded in docs/adjudication/DIVERGENCE_REGISTER.md,
and this T&F CSE oracle comparison is concrete evidence it diverges
from real citeproc-js output for a style that doesn't request
collapsing.

## Needs adjudication, not just a fix
- [ ] Determine whether Citum's always-collapse behavior should
      become conditional on a style-level setting (mirroring CSL's
      collapse attribute more faithfully), or whether this stays a
      deliberate Citum design principle that gets registered as an
      accepted divergence (matching the pattern of other entries in
      DIVERGENCE_REGISTER.md, e.g. div-014).
- [ ] If conditional: design the schema field, default it to
      preserve csl26-dpep's current behavior for styles that don't
      set it explicitly (or default to opt-in per real CSL
      semantics -- this is the actual adjudication question).
- [ ] Either way, add a DIVERGENCE_REGISTER.md entry.
- [ ] Re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date
      after the decision lands to confirm impact.

Related: csl26-dpep (the engine change establishing this behavior).

## Adjudication (2026-08-18)

Resolved: same-author collapse becomes conditional on a style-level setting,
mirroring CSL's `collapse` attribute more faithfully — not registered as an
accepted divergence.

Spec: `docs/specs/SAME_AUTHOR_COLLAPSE.md` (Draft, PR pending). Extends the
existing `citation.collapse` field (today: `citation-number` only) with a
`same-author` variant carrying its own `year-suffix` sub-setting
(`separate`/`merged`/`ranged`), rather than adopting CSL's flat four-value
enum or a regime-derived boolean — see the spec's Rejected Alternatives for why.

Corpus measurement (2 844 independent `styles-legacy/*.csl`): 763 files declare
no `collapse` attribute at all and Citum collapses them anyway today; of those,
361 are note-regime (this is the `csl26-m11m` connection) and 44 are numeric
(collapse invisible there). The other 1 165 `year*`-declaring styles migrate
today via `extract_citation_collapse`'s `_ => None` arm, which silently
discards the `year-suffix`/`year-suffix-ranged` distinction — the new field
maps all four CSL values losslessly.

`div-017` (comma-vs-semicolon on no-locator same-author collapse) is unaffected:
`chicago-author-date.csl` declares `collapse="year"`, so
`chicago-author-date-18th` still opts in under the new field and still
collapses.

Implementation is a separate, stacked PR (schema change, needs the docs PR
reviewed first per repo policy). `csl26-m11m` is blocked-by this bean and
closes as a consequence — no note-specific code needed, note styles simply
never declare `collapse`.

## PR 2 Implementation Plan

Tracked in `/home/bruce/.claude/plans/in-the-plan-home-bruce-claude-plans-comp-reactive-pancake.md`.

- [x] Commit 1: feat(schema)! — CitationCollapse::SameAuthor + regime validation + schema-gen
- [x] Commit 2: feat(migrate) — extract_citation_collapse maps all four CSL values
- [x] Commit 3: feat(styles) — declare collapse on 9 tracked styles
- [x] Commit 4: feat(engine)! — gate group_citation_items_by_author on collapse setting
- [x] Commit 5: test(engine) — pin non-collapsed clusters + schema round-trip/rejection tests
- [x] Commit 6: docs(spec) — SAME_AUTHOR_COLLAPSE.md v1.0->v1.1 Active; CITATION_CLUSTER_RENDERING.md v1.5
- [x] Commit 7: chore(beans) — close csl26-ecfn/csl26-m11m, filed 4 follow-up beans (csl26-ctkb, csl26-llgj, csl26-8g14, csl26-ax22)
- [x] Verification: nextest after commit 4 (0 fail), pre-commit gate (2629/2629), schema-gen, load-validate all styles, per-style report-core (9 zero-movement, 4 gains matching prediction exactly), snapshots clean, MLA confirmed moot, guarded corpus sweep (net +6, zero regressions across 24 families)
- [ ] gh stack submit (after user confirmation), gh pr checks --watch

## Summary of Changes

Implemented in PR 2 (stacked on the docs PR #1206), commits on
`feat/csl26-ecfn-collapse-opt-in`:

1. **Schema**: `CitationCollapse` gains `SameAuthor(SameAuthorCollapse)`
   with a `year_suffix` degree (separate/merged/ranged). Hand-written
   serde mirrors `Processing`'s existing pattern. Regime-coherence
   validation added (same-author on AuthorDate/Note/Custom,
   citation-number on Numeric/Custom), hooked into both style-resolution
   return paths.
2. **Migrate**: `extract_citation_collapse` now maps all four CSL
   `collapse` values losslessly, dropping the mapped value if illegal for
   the detected regime rather than emitting something invalid.
3. **Styles**: 9 tracked styles declare `collapse: { same-author: {} }`
   (config-map form, not bare-string sugar — the generated schema only
   advertises the object form). Table mechanically re-derived against
   every tracked style's real `styles-legacy/` source, correcting the
   docs PR's 5-style enumeration to 9.
4. **Engine**: `group_citation_items_by_author` gates same-author
   merging on `collapse == Some(SameAuthor(_))`; singleton groups
   otherwise, keyed on `(index, id)` so duplicate-id clusters can't
   spuriously merge. Gated at the grouping level (not just the collapse
   branch) so the integral prose-anchor path is covered too.
5. **Tests**: native-fixture pins for the note-regime fix (byte-exact
   against citeproc-js), duplicate-id regression, shortened-notes,
   T&F CSE / MLA no-collapse cases, plus schema round-trip and
   regime-coherence tests.

**Measured**: full `report-core.js` sweep of all 24 core-style families
(baseline: PR1 tip `52481a17`) shows net **+6** exactParity, **zero
regressions**. The 9 opt-in styles show exactly zero movement.
`chicago-notes-18th` 22/72 → 23/72, `chicago-shortened-notes-bibliography`
60/473 → 61/473, `taylor-and-francis-council-of-science-editors-author-date`
30/67 → 32/67, `modern-language-association` 42/115 → 44/115.

Resolves `csl26-m11m` as a direct consequence — closed separately with its
own summary.
