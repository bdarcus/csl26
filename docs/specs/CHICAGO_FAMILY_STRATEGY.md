# Chicago Family Strategy: Authority, Metrics, and Cluster-Driven Tuning

**Status:** Active
**Version:** 1.0
**Date:** 2026-08-07
**Supersedes:** (none — refines `csl26-40n4`/`csl26-h7oc` execution, does not replace them)
**Related:** [`docs/architecture/audits/2026-06-30_CHICAGO_FAMILY_AUDIT.md`](../architecture/audits/2026-06-30_CHICAGO_FAMILY_AUDIT.md), [`CHICAGO_18_COVERAGE.md`](./CHICAGO_18_COVERAGE.md), [`CONTRIBUTOR_PHRASE_MESSAGES.md`](./CONTRIBUTOR_PHRASE_MESSAGES.md), [`LOCALE_MESSAGES.md`](./LOCALE_MESSAGES.md), beans `csl26-40n4`, `csl26-h7oc`, `csl26-dfq0`

## Purpose

Three problems in the Chicago-family effort needed a stated resolution before
more tuning work landed: which authority governs when CMOS-18 prose and
citeproc-js output disagree, what fidelity means relative to exact parity, and
why the per-entry tuning loop (`csl26-giun`) has repeatedly deferred the same
structural clusters across roughly six sessions. This spec answers all three
and replaces the informal framing scattered across `csl26-40n4`'s child beans.

## Scope

In scope: the authority rule for CSL-derived Chicago-family styles; the
relationship between the fidelity and exact-parity metrics; the cluster-driven
execution model that replaces per-style/per-entry tuning; what the 2026-06-30
audit got right and one place it needs correction.

Out of scope: implementing any specific cluster (tracked per-cluster in child
beans under `csl26-h7oc`); engine/Rust changes; re-migrating any style from
legacy CSL.

## Design

### Authority rule

For CSL-derived styles, including the entire Chicago family, **citeproc-js
output is the target authority**. CMOS-18 prose — including secondary
summaries of it, such as an LLM-generated rule list — is useful for
*explaining and adjudicating* a divergence once one is found. It is never used
to *set* a rendering target. Where a CMOS claim contradicts oracle output,
parity wins and the divergence is registered in
`scripts/report-data/known-divergences.json`, not silently overridden.

This matters because CMOS-18 prose summaries are easy to get wrong at the
level of detail styles actually need. Verification method: grep the legacy CSL
source for actual `<text variable="…">` **render** sites, not `<if
variable="…">` conditionals — conditionals are guard clauses and vastly
over-count (17 `publisher-place` hits in `chicago-author-date.csl` vs. 3 real
render sites, two of them commented out).

A worked example, from checking a secondary CMOS-18 summary against
`styles-legacy/chicago-author-date.csl`:

| Claim | Verdict | Evidence |
|---|---|---|
| Place-of-publication omitted by default | Holds | 3 render sites, 2 commented out (lines 3266, 3288) |
| Et al. thresholds identical across citation and bibliography | **False** | Same style: `et-al-min="3"` citation (3996) vs. `et-al-min="7"` bibliography (4042) |
| DOI preferred over URL | Holds | line 3627: `<if variable="DOI">` … `<else-if variable="URL">` |
| Access dates omitted unless no reliable pub/revision date | Not yet verified | `accessed` term renders at 3617; conditional not traced |
| Headline-style capitalization | Not yet verified | interacts with open bug `csl26-4kt3` (text-case token preservation) |

Unverified rows are not adopted as targets until checked this way, during the
cluster work that touches them — not as an up-front audit pass.

### Fidelity vs. exact parity

`scripts/report-core.js`'s `compatibilityScore` is literally assigned from
`fidelityScore` (report-core.js:2694), and `computeFidelityScore`
(report-core.js:1794) is a normalized-text pass/fail rate — strictly weaker
than exact parity, computed against the same citeproc-js oracle. It carries no
information exact parity doesn't already carry. It is also already non-binding
for this family: the hard `fidelityScore === 1.0` gate in
`check-core-quality.js` only applies to styles listed in
`core-quality-baseline.json`, which excludes all four Chicago variants.

**Exact parity is the stated target for this family.** Fidelity is retained as
a coarse regression tripwire (see `check-core-quality.js`'s existing
`min_pass_rate` floors in `verification-policy.yaml`) but is no longer
presented as an independent quality dimension. This is a re-framing of
existing measurement, not a rewiring of it — `computeFidelityScore` and
`min_pass_rate` are unchanged; only how the numbers are presented changes (see
the reporting changes tracked under `csl26-dfq0`).

### What the 2026-06-30 audit got right, and one correction

The [Chicago Family Audit](../architecture/audits/2026-06-30_CHICAGO_FAMILY_AUDIT.md)
classified shared rules into three buckets. Section A (shared component
candidates) and Section B (order-layer, must stay separate) both hold and are
implemented: `chicago-18-base.yaml` carries exactly the Section A items
(page-range format, punctuation-in-quote, demote-non-dropping-particle,
multilingual), and `chicago-author-date-18th`/`chicago-notes-18th` both extend
it while keeping their citation grammars and bibliography ordering fully
separate, per Section B. **Neither changes as part of this strategy.**

Section C ("missing conversion/accessor facts — a Rust, not YAML, problem") has
since been shown to be **misdiagnosed**, per the investigation recorded on the
now-scrapped bean `csl26-ifhx`: the accessors it flagged as missing already
exist —archival fields (`accessors.rs:777-852`), original-publication dates
(`accessors.rs:1332-1349`), event dates (`conversion/scholarly.rs:381`),
note-derived roles (`conversion/mod.rs:46-57`), recordings/broadcasts
(`fixups/media.rs:186`). The audit's method inspected only the Chicago YAML
type-variants and mis-attributed *template gaps* (a type-variant simply not
referencing an accessor that exists) to *accessor gaps*. There is no
outstanding Rust work here; the entire remaining Chicago-family gap is YAML
template wiring. `csl26-ifhx` was scrapped on this finding and its scope
absorbed into `csl26-h7oc`.

Separately, `CHICAGO_18_COVERAGE.md` (Active) proposed and delivered schema
additions — `Event`, `WorkRelation`, contributor roles including `narrator` and
`performer` — via the completed bean `csl26-ccyy`. Those fields exist and are
available to templates; using them is, again, YAML wiring, not new schema work.

### Cluster-driven execution (replaces per-style tuning)

`csl26-giun` (author-date) and `csl26-7jht` (shortened-notes) are structured
one bean per *style*, but nearly every entry in their multi-session histories
names the same handful of structural defects — broadcast/episode grammar,
multi-volume chains, legal citations, patents, original/reprint trailers —
deferred repeatedly because each pass chased the *next failing entry* within
one style rather than a *defect class* across the family. A defect class fixed
in one style's template usually recurs verbatim in its siblings (T&F inherits
author-date's gaps directly; shortened-notes shares notes-18th's citation
baseline).

Going forward, `csl26-h7oc`'s children are **defect clusters**, not styles.
Each cluster bean names the source-type or component pattern, lists every
style it touches, and is checked against exact parity across all four
variants at once. Ordered by residual observation count from the `csl26-6th8`
classification:

1. Contributor-role and role-pattern rendering (localization; this strategy's
   first proof, see below)
2. Title quoting boundary by source type
3. Container-title terminal punctuation before volume/issue
4. Name-list conjunction punctuation
5. Archival / manuscript / `document`-routed refs
6. Broadcast & episode grammar
7. Multi-volume chains, legal, patents, original/reprint trailers

A cluster that does not move parity upward across the styles it touches gets
reverted, not argued for — this is the enforcement mechanism that replaces
"deferred, revisit later."

### Localization as a first-class defect class

Auditing `chicago-author-date-18th.yaml` and
`taylor-and-francis-chicago-author-date-core.yaml` surfaced that localization
adoption is wildly uneven *within this one family*, and that no existing
metric — fidelity or exact parity — can see it, because the oracle is
English-only:

| File | `message:` (locale-driven) uses | hardcoded English prose |
|---|---|---|
| `chicago-notes-18th` | 17 | 7 |
| `chicago-shortened-notes-bibliography-core` | 5 | 1 |
| `chicago-author-date-18th` | 8 | 27 |
| `taylor-and-francis-chicago-author-date-core` | 0 | 17 |

This is not a missing-capability problem. `chicago-notes-18th.yaml:288` already
calls `message: pattern.chicago-written-by`, and the locale already ships eight
`pattern.chicago-*` messages authored specifically for this family
(`chicago-aired-date`, `chicago-by`, `chicago-interview-by`, `chicago-on`,
`chicago-review-of`, `chicago-to`, `chicago-with`, `chicago-written-by`) plus
`verb`/`verb-short` forms on the relevant contributor roles. It is a porting
job with a working in-repo reference implementation, tracked as cluster 1
under `csl26-h7oc` and bean `csl26-dfq0`.

This is the *simple compositional message* class that PR #966 already solved
(single role, single rendered name-list, fixed phrase — `{$name}` or no
argument at all). It is explicitly **not** the class of problem
`CONTRIBUTOR_PHRASE_MESSAGES.md` (Draft) addresses — that spec's
`pattern.in-contributor-container` / `pattern.container-contributor-title`
exist for phrases where role labels, contributor lists, and a rendered
title/container fragment must be jointly reordered by locale. None of the
Chicago-family sites in cluster 1 need that; they resolve entirely within the
already-shipped message model. A future cluster that hits joint
name/title/container reordering should use that Draft spec's mechanism instead
of extending the simple pattern model past its design.

See `docs/policies/LOCALIZATION_INTEGRITY.md` for the binding rule this
finding produces, and `STYLE010` in `scripts/style-structure-lint.js` (bean
`csl26-dfq0`) for how it is measured — the existing deterministic style-shape
linter, not the oracle-driven fidelity/SQI pipeline, per `docs/reference/SQI.md`'s
documented separation between the two. Report-only for now (not in
`FATAL_RULE_IDS`), so it disturbs neither existing SQI nor fidelity scores.

## Implementation Notes

- `chicago-18-base.yaml` is not touched by any cluster in this plan.
- Cluster child beans replace `csl26-giun`/`csl26-7jht` as the unit of tuning
  work; those two beans are closed with pointers rather than deleted, to
  preserve their investigation history.
- DOI/URL literal-prefix duplication (`"https://doi.org/"`, ~15 sites in this
  family) is a related but separate defect class — declarative `links:`
  config already exists and is used by other embedded styles (`LinkTarget`,
  `citum-schema-style/src/options/mod.rs:605-655`) — tracked in `csl26-629e`,
  not a cluster under this spec.

## Acceptance Criteria

- [ ] All four Chicago-family styles' exact parity moves upward from the
      2026-08-07 baseline (author-date 172/546, T&F 172/546, notes 22/72,
      shortened-notes 13/473) with each cluster landed — cluster 1 held both
      at 172/546 (zero entries changed either direction, verified
      entry-by-entry, not just aggregate counts); it was a pure localization
      pass, not a fidelity lift, by design. Cluster 2 moved author-date and
      T&F to 173/546 (article-newspaper quoting fix, inherited by T&F for
      free); notes-18th and shortened-notes-bibliography unchanged at 22/72
      and 13/473 (defect didn't reach them this pass — see csl26-87yl)
- [x] `csl26-h7oc` is restructured to cluster-shaped children per the ordered
      list above
- [x] Cluster 1 (contributor-role localization) lands with a verified
      multilingual render proof (`citum render refs -L fr-FR`/`-L de-DE`:
      narrator "Narrated by" → "Lu par"; translator "Translated by" →
      "Übersetzt von" (de-DE) / "Traduit par" (fr-FR, T&F))
- [x] No embedded or in-repo style outside the Chicago family regresses in
      exact parity as a side effect of locale additions made for this family
      (`check-core-quality.js` against the 19-style embedded-core baseline:
      gate passed, 0 exact-parity regressions; only unrelated pre-existing
      `ieee` preset-usage warning, untouched by this stack)
- [x] `docs/reference/SQI.md` and the generated compat dashboard reflect
      exact parity as the headline metric and fidelity as a tripwire

Cluster 1 also surfaced and fixed a real schema gap outside its original
scope, per explicit instruction not to defer it: `citum-schema-style::template::ContributorRole`
was missing a `Narrator` variant (had `Performer`/`Illustrator`/`Writer` but
not `Narrator`), so `contributor: narrator, form: verb` silently rendered no
role label at all. Fixed with a 4-line Rust diff (template.rs, mod.rs,
substitute.rs, raw_conversion.rs); full `cargo nextest run` (2420/2420),
`cargo clippy -D warnings`, and `just schema-gen` all clean.

## Changelog

- 2026-08-07: Initial version.
- 2026-08-07: Cluster 1 (contributor-role localization) landed for
  `chicago-author-date-18th` and `taylor-and-francis-chicago-author-date-core`;
  acceptance criteria updated with verified results.
- 2026-08-07: Cluster 2 (title quoting boundary) landed `article-newspaper`
  and `thesis` quoting fixes for `chicago-author-date-18th`, inherited by
  `taylor-and-francis-chicago-author-date-core`. `map` and the
  `chicago-notes-18th`/`chicago-shortened-notes-bibliography-core`
  `dataset`/`report`/`thesis`/`webpage` entanglement deferred to future work
  — see csl26-87yl for the evidence. Confirms the LOC-reduction half of the
  originating commit's framing is not being pursued by the cluster plan as
  scoped: `chicago-18-base.yaml` stays untouched by design (per Implementation
  Notes above), so gains come from the existing extends chain propagating a
  fix for free, not from new shared abstraction.
