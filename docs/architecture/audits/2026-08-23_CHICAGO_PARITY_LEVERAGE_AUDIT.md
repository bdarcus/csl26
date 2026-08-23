# Chicago Parity Leverage Audit — Defect Classes, Not Styles

- **Date:** 2026-08-23
- **Bean:** `csl26-h7oc` (epic: drive all Chicago variants to full fidelity)
- **Scope:** `chicago-author-date-18th`, `chicago-notes-18th`,
  `chicago-shortened-notes-bibliography(-core)`,
  `taylor-and-francis-chicago-author-date(-core)` — the four embedded Chicago
  18 variants in `crates/citum-schema-style/embedded/styles/`.
- **Trigger:** [PR #1218](https://github.com/citum/citum-core/pull/1218) spent
  most of a session on the family and moved `chicago-author-date-18th` exact
  parity 174 → 191 of 542 (+17, +3.1pt). That rate does not reach the
  `csl26-w0hf` milestone (100% embedded parity) in any reasonable time.
  Question raised: is the style-evolution process itself flawed?
- **Answer:** yes, but not in the way the low numbers suggest. The residual is
  not a long tail of unique bugs and is not concentrated in exotic reference
  types. It is roughly ten defect classes with heavy multiplicity, sitting
  mostly on the highest-volume types, and the current process selects work by
  narrative cluster identity rather than by measured residual count. This
  audit computes, for the first time, the leverage table that should have been
  driving cluster selection.

Numbers below are measured against the PR #1218 final branch state
(`8893cd01`, the content merged to `main` at `05f328a0`), using cached
`report-core.js --style <name> --all-features` output. Re-running against
current `HEAD` will shift individual counts by a handful of entries (unrelated
commits landed since); it will not change any conclusion here.

## Method

For each of the four styles, every row in `oracleDetail` (bibliography, gated
by `exactParityEligible`) and every row in `citationEntries` that fails exact
match was pulled and diffed word-by-word against the oracle string
(`difflib.SequenceMatcher`, non-`equal` opcodes only). Each failing row was
then multi-labeled against ~19 pattern rules (quote boundaries, case
transitions, date-detail tokens, contributor-role phrases, genre/medium
vocabulary, volume/issue grammar, legal-citation tokens, URL/DOI presence,
punctuation-only regions, and so on) — a row commonly carries more than one
label, since e.g. a missing subtitle also shifts the quote-close position.
Reference types were joined from `tests/fixtures/test-items-library/chicago-18th.json`
(the 403-item CMOS-18 corpus this family is authored against) and the shared
`references-expanded.json` fixture. A greedy set-cover then ranks labels by
how many additional rows they fully explain once already-chosen labels are
subtracted — this is the leverage order: fixing the classes at the top of the
list, in full, flips the most rows to exact match per unit of work.

This is the exact-parity analogue of the fidelity-oriented
`scripts/analyze-oracle-clusters.py`, whose own docstring already states the
intended method: *"before spending effort on entry-by-entry fixes … triage the
resulting clusters biggest-first."* That triage has never been run against
exact parity for this family; this audit is a one-time manual pass, and
`scripts/analyze-parity-residuals.js` (landed alongside this audit) makes it
repeatable per style, on demand.

## 1. Where things actually stand

### Family totals (divergence-adjusted exact parity)

| Style | Exact parity | Fidelity |
|---|---|---|
| `chicago-author-date-18th` | 191/542 (35%) | 0.904 |
| `taylor-and-francis-chicago-author-date` | 191/542 (35%) | 0.904 |
| `chicago-notes-18th` (citations only) | 28/72 (39%) | 0.959 |
| `chicago-shortened-notes-bibliography` | 81/473 (17%) | 0.769 |
| **Family** | **491/1,629 (30%)** | |

1,148 failing rows across the family: 1,059 bibliography, 89 citation.
`chicago-notes-18th` has no bibliography rows at all — it is a citation-only
style — and contributes 46 of the 89 failing citation rows. Its own citation
grammar is the entirety of its identity, so this is where the family's
citation-side residual concentrates.

### The failures are on the bread-and-butter types, not the exotic tail

Per-type exact parity for `chicago-author-date-18th`, measured against the
403-item CMOS-18 corpus:

| Type (count) | Parity | Type (count) | Parity |
|---|---|---|---|
| book (125) | **52/125 — 42%** | thesis (6) | 0/6 |
| article-journal (76) | **45/76 — 59%** | post (6) | 0/6 |
| document (35) | 0/35 | map (5) | 0/5 |
| article-newspaper (22) | 1/22 | standard (5) | 0/5 |
| chapter (19) | 7/19 | software (5) | 0/5 |
| article-magazine (17) | 6/17 | graphic (4) | 0/4 |
| webpage (16) | 1/16 | entry-dictionary (4) | 0/4 |
| manuscript (15) | 2/15 | hearing (3) | 0/3 |
| legislation (11) | 0/11 | treaty (2) | 0/2 |

`book` and `article-journal` are 37% of the corpus by count and each fail more
than a third of the time. Even a program that drove every zero-scoring rare
type (`document`, `article-newspaper`, `legislation`, `thesis`, `map`,
`standard`, `software`, `graphic`, `entry-dictionary`, `hearing`, `treaty`, and
more) to 100% would leave the family under 50%, because those types are a
minority of the corpus. The leverage is in the common types, not the tail.

`chicago-shortened-notes-bibliography` shows the same shape at lower absolute
parity (`book` 26/119 — 22%; `article-journal` 14/61 — 23%), consistent with
it sharing the same underlying template defects plus its own note-flow
grammar gaps.

### Ten defect classes explain 69% of every failing row

Multi-label classification of all 1,148 failing rows, followed by greedy set
cover:

| # | Defect class | Rows carrying it | Sole cause | Cumulative rows flipped |
|---|---|---|---|---|
| 1 | punctuation / delimiter only | 221 | 77 | 77 (6.7%) |
| 2 | date detail (month/day) dropped | 264 | 41 | 138 (12.0%) |
| 3 | title quote boundary | 300 | 50 | 221 (19.3%) |
| 4 | **title case not applied** | 255 | 37 | **383 (33.4%)** |
| 5 | genre / medium label | 82 | 37 | 449 (39.1%) |
| 6 | volume / issue / series grammar | 136 | 20 | 519 (45.2%) |
| 7 | contributor role & ordering | 143 | 37 | 606 (52.8%) |
| 8 | year-suffix letter | 97 | 37 | 682 (59.4%) |
| 9 | URL / DOI policy | 89 | 22 | 747 (65.1%) |
| 10 | title case over-applied / stop word | 49 | 15 | **796 (69.3%)** |

("Rows carrying it" and "cumulative rows flipped" differ because most rows
carry 2+ labels; set cover credits a row to the cumulative total only once
every label it carries has been added to the chosen set.)

Projected per-style parity if all ten land in full — optimistic, since these
are triage buckets rather than confirmed single root causes per row:

| Style | Now | Projected |
|---|---|---|
| `chicago-author-date-18th` | 191/542 (35%) | 424/542 (78%) |
| `taylor-and-francis-chicago-author-date` | 191/542 (35%) | 424/542 (78%) |
| `chicago-notes-18th` | 28/72 (39%) | 62/72 (86%) |
| `chicago-shortened-notes-bibliography` | 81/473 (17%) | 377/473 (80%) |
| **Family** | **491/1,629 (30%)** | **1,287/1,629 (79%)** |

A further 184 rows land in an "unclassified" bucket that the 19 pattern rules
don't catch. Sampling that bucket surfaces at least six more named classes,
none yet tracked anywhere:

- `section` variable never rendered (`"Sports. New York Times"` where the
  oracle prints the newspaper section before the outlet).
- edition ordinal not formatted (`"2."` where the oracle has `"2nd ed."`).
- publisher-place parenthetical dropped (`"(New York)"`).
- `title-short` leaking into a bibliography slot that should carry the full
  title (`"Targeting"` for `"Targeting Using Differential Incentives"`).
- unsupported primary-contributor roles collapsing the whole name list
  (`"Austin, Tim, comp."` in the oracle vs. the entry falling back to a
  title-first form in Citum, as if no contributor existed at all).
- inverted sort/name order for name particles (`"De Cervantes Saavedra,
  Miguel"` where Chicago keeps the particle with the given name:
  `"Cervantes Saavedra, Miguel de"`).

### Sub-splitting the title-case class — do not fix this blindly

The 255-row title-case class is three different roots, and one sub-root is a
**recorded, deliberate divergence** that must not be reverted while fixing the
other two:

- **(a) 182 entries — title case simply not applied.**
  `"Mesopotamia: between two rivers"` where the oracle has `"Mesopotamia:
  Between Two Rivers"`; `"Meaning and understanding in the history of ideas"`
  vs. `"Meaning and Understanding in the History of Ideas"`. This is YAML
  wiring — the type-variant in question does not carry the case transform at
  all. It is the single largest individual defect found in this audit.
- **(b) 31 entries — over-capitalized.** Two distinct sub-causes: genuine
  stop-word gaps against citeproc-js's `skipWordsRex` (`in`, `into`, `via` —
  8 entries, e.g. `"Language and Design In Pippa Passes"` should be
  `"...in Pippa Passes"`), and type-variants applying title case to slots
  where CSL applies none at all — `post` and `article-newspaper` headlines are
  sentence case in the oracle, not title case (23 entries).
- **(c) 2 entries — acronym / mixed-case regressions** (`PhD` rendering as
  `Phd`). This sits directly adjacent to the completed, intentional divergence
  recorded in `docs/policies/TEXT_CASE_PROTECTION.md` (bean `csl26-4kt3`,
  internal-caps preservation under sentence case). That policy is correct and
  is not this audit's target; whatever fixes (a)/(b) must not touch the
  internal-caps preservation mechanism.

## 2. Why the process produced +17 in a day

`docs/specs/CHICAGO_FAMILY_STRATEGY.md`'s cluster model is directionally
right — its cluster 2 (title quoting boundary) *is* the top single measured
class here at 300 rows. Four specific things defeated it in practice:

1. **Clusters were ranked by narrative, not by measured count.** Cluster 1
   (contributor-role localization) was ranked first and, by the spec's own
   stated design, moved parity by exactly zero entries — "a pure localization
   pass, not a fidelity lift, by design." Meanwhile title case (255 rows),
   date detail (264 rows), and terminal punctuation (221 rows) — three of the
   top five measured classes in this audit — are not clusters in the spec and
   have no beans at all.

2. **Clusters, once identified, were executed one source-type at a time.**
   Cluster 2 (title quoting) landed `article-newspaper` and `thesis` fixes,
   moved `chicago-author-date-18th` and the T&F sibling +1 entry each, and
   explicitly deferred `map`, `dataset`, `report`, `webpage` to `csl26-87yl` —
   see that spec's changelog. A 300-row class was worked at roughly one row
   per pass.

3. **The `style-tune` loop's ordering and stop condition make that a legal
   place to stop.** `.claude/skills/style-tune/SKILL.md` runs the lenient
   fidelity gate to 100% first, then the exact-parity loop, and explicitly
   permits a pass to land "with residual exact-parity gap if every residual is
   classified (fixed, escalated as `unclear`, or excluded via a registered
   divergence)." For this family, fidelity is already 0.90–0.96 on three of
   four styles while exact parity is 0.17–0.39 — the loop spends its ordering
   priority on the metric that is nearly saturated and is structurally
   permitted to stop on the one that is not.

4. **The adjudication ledger has never been used, and it isn't wired to the
   gate either.** `scripts/report-data/parity-adjudication.json` carried
   `"entries": {}` before this audit. The 100%-parity denominator has
   therefore never been checked for rows that may be unwinnable by either
   side. This audit found one class that qualifies (§3 below) and one class
   that looked like it might but does not (§3). Populating the ledger also
   surfaced a second gap: `check-core-quality.js` validates entry shape and
   reports `unclear`/`citum-correct`/`citeproc-correct` counts, but neither
   it nor `report-core.js` currently subtracts adjudicated rows from
   `exactParity.total`. The ledger the workflow docs instruct agents to write
   to has no consumer that acts on it yet — writing an `unclear` entry today
   records a decision but does not change any reported number. Wiring that up
   is a `report-core.js` change and is out of scope for this docs-only audit;
   flagging it here so it isn't mistaken for already-solved.

5. **Bean sprawl diffuses effort.** 177 beans reference Chicago; 13 were
   simultaneously `in-progress` at the time of this audit. There was no bean
   for the largest measured defect in the family (title case not applied,
   182 entries) prior to this audit.

## 3. Adjudication findings

Two candidate classes for `scripts/report-data/parity-adjudication.json` were
evaluated; only one qualifies.

**Genre-slug divergence — recorded as `unclear`.** A cluster of items in
`references-expanded.json` (not the CMOS-18 corpus) carry `genre` as a
kebab-case slug — `phd-thesis`, `short-film`, `assessment-report`,
`manuscript-scroll`, `holograph-manuscript`. citeproc-js echoes the literal
slug capitalize-first (`Phd-thesis`, `Short-film`); Citum humanizes it
(`PhD thesis`, `Short film`). The real 403-item CMOS-18 corpus stores `genre`
as free prose written the way a cataloger would write it (`"PhD diss."`,
`"working paper"`, `"telegram"`) — so this divergence is an artifact of how
the synthetic `references-expanded.json` fixture encodes `genre`, not evidence
about either processor's correctness. Recorded as `unclear` per
`scripts/report-data/parity-adjudication.json`'s stated semantics ("neither
side's correctness is established from available evidence"); excluded from
the gate denominator pending your review. The fixture-repair option (writing
`genre` as prose in `references-expanded.json` instead) is out of scope here —
that fixture is shared across the whole embedded-tier suite and a prior edit
to two of its entries regenerated 2,845 oracle snapshots, so any repair needs
its own scoped change.

**CMOS-18 corpus-annotation rows — not adjudicated, left in the fixable
pool.** ~12 `document`-typed items in the CMOS-18 corpus have titles that are
editorial commentary about the corpus itself rather than citable works —
`"CMOS erroneously places the date of this example before the place."`,
`"CSL needs a container-genre."`, `"CSL has no way to indicate whether a
translator should be connected to the prim[ary title]."` (titles truncated
mid-sentence in the source fixture). These initially looked like adjudication
candidates. They are not: this audit's own leverage table shows they flip
under the title-case and title-quoting waves (classes 3–4 above) exactly like
any other `document` row — citeproc-js's output for them is well-defined and
matchable, so `unclear`'s "neither side's correctness is established" does not
apply. The question they actually raise is different — **should test-suite
errata about the CMOS-18 corpus itself count as citable items in a parity
gate at all?** — a corpus-membership question, not a parity adjudication. That
question is raised here for you rather than answered by this audit; the rows
stay in the fixable pool in the meantime.

## Forward plan

Not executed by this audit — scoped for review before any style or engine
change lands. Ordered by set-cover contribution, with the expected implementation
layer named up front so a `processor-defect` doesn't get treated as a YAML fix:

| Wave | Class | Expected layer | Rows |
|---|---|---|---|
| 1 | title case not applied (a) + stop-word/over-applied (b) | YAML type-variants (4 files) + engine stop-word list | 304 |
| 2 | title quote boundary — all source types at once, not one at a time | YAML type-variants | 300 |
| 3 | date detail (month/day) | mixed: date-part wiring per type-variant | 264 |
| 4 | punctuation / delimiter | mixed | 221 |
| 5 | contributor role & ordering (incl. `comp.`/`trans.` as primary roles) | engine role support + YAML order | 143 |
| 6 | volume / issue / series grammar | YAML | 136 |
| 7 | year-suffix letter (wrong letter; leaks into date ranges) | **engine** | 97 |
| 8 | URL/DOI policy, genre/medium label, edition ordinal, `section` | mixed | ~190 |

Carried unchanged from the existing strategy: a wave that does not move parity
across every style it touches is reverted, not argued for. Each wave should
report before/after per-type parity for `book` and `article-journal`
specifically, not only family totals — those two types are 37% of the corpus
and the numbers most likely to mask a regression hiding inside an aggregate
gain.

## Acceptance criteria for the forward plan

Stated as per-type floors, because family totals hid the shape of this problem
for at least one full PR cycle:

- `chicago-author-date-18th`: `book` ≥ 100/125 and `article-journal` ≥ 68/76
  after waves 1–4; style total ≥ 400/542.
- `chicago-shortened-notes-bibliography`: total ≥ 350/473.
- `chicago-notes-18th`: citation parity ≥ 60/72.
- `taylor-and-francis-chicago-author-date` tracks `chicago-author-date-18th`
  within 2 entries at every checkpoint (it inherits by `extends:`; a wider gap
  means a fix landed in the wrong file).
- No non-Chicago embedded style regresses:
  `node scripts/check-core-quality.js --parity-baseline scripts/report-data/embedded-parity-baseline.json`.

## Postscript: wave 1 measured result (added after landing)

Wave 1 (title case) executed, verified with per-entry `exactMatch` diffing
(not just aggregate counts) plus the full `cargo nextest run` (2,691/2,691
passed): **+7 net entries family-wide** (491/1,629 → 498/1,629), against
this audit's own 304-row projection for the class. Zero regressions in the
landed version — but the first attempt was not zero-regression: adding the
full author-date-18th type-mapping list (`broadcast`/`collection`/
`manuscript`/`motion-picture`/`song`/`webpage`) to `chicago-notes-18th` and
`chicago-shortened-notes-bibliography-core` regressed 3 previously-passing
archival/manuscript entries, because those types picked up the same
`component: quote: true` that correctly applies to genuine article titles
but not to bare collection titles ("Revere Family Papers", "Landscapes of
Zambia, Central Africa"). The fix landed narrower — only `map`/`dataset` —
with the rest deferred to wave 2.

Two things this confirms about the audit's own method, worth stating
plainly rather than leaving implicit:

- **The 10-class table's projections are triage-bucket estimates, not
  confirmed yields, exactly as this audit already flagged for the
  title-case sub-split.** The gap between +304 projected and +7 measured is
  almost entirely the A1/title-case and B/quote-boundary classes turning
  out to be far more entangled than the multi-label counting could see:
  `map`/`dataset` in `chicago-author-date-18th` and
  `taylor-and-francis-chicago-author-date` moved from "wrong case" to
  "right case, wrong quoting" — correct progress, zero visible movement in
  the gate, because both defects must clear on the same entry before it
  counts. Wave-by-wave execution should expect this pattern generally, not
  just for title case: treat each wave's projection as an upper bound
  contingent on the *next* wave, not an independent number.
- **Aggregate delta is not sufficient regression evidence.** The rejected
  first attempt looked like a clean net win (+5 shortened-notes-bibliography
  bibliography entries) and was quietly hiding 3 regressions until per-entry
  pass/fail comparison caught it. `docs/guides/STYLE_WORKFLOW_EXECUTION.md`'s
  exact-parity loop is amended (see its own changelog) to require this
  check explicitly, prompted by this finding.

## Related work

- `docs/specs/CHICAGO_FAMILY_STRATEGY.md` (Active) — the cluster-driven
  execution model this audit refines; not superseded, its authority rule and
  fidelity/exact-parity framing hold.
- `docs/architecture/audits/2026-06-30_CHICAGO_FAMILY_AUDIT.md` — the prior
  shared-substrate classification; unaffected by this audit's findings.
- `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md` — the
  fidelity-vs-exact-parity distinction this audit's method depends on.
- `docs/policies/TEXT_CASE_PROTECTION.md` — the case-transform policy that
  wave 1 must not regress.
- `scripts/analyze-parity-residuals.js` — the repeatable tool this audit's
  method has been ported into (landed alongside this audit; see the tune-loop
  ordering change in `docs/guides/STYLE_WORKFLOW_EXECUTION.md`).
- Beans: `csl26-h7oc` (epic), `csl26-40n4` (substrate), `csl26-w0hf`
  (milestone), `csl26-dfq0` (strategy reset), `csl26-87yl` (deferred cluster-2
  entries this audit's wave 2 absorbs).
