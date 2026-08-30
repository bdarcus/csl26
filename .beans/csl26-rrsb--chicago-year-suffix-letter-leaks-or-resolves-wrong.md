---
# csl26-rrsb
title: 'Chicago: year-suffix letter leaks or resolves wrong'
status: in-progress
type: task
priority: high
tags:
    - engine
    - chicago
    - fidelity
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-08-30T14:08:44Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 97 entries where a disambiguation year-suffix letter (2019a/2019b) is wrong, missing, or leaks into an adjacent date range. Flagged as an engine-layer defect, not YAML -- classify with the conversion-layer pre-flight (docs/policies/STYLE_WORKFLOW_DECISION_RULES.md) before touching any style file. Touches all four Chicago variants.

## Progress: root-caused two distinct engine defects (2026-08-30)

Reproduced against `chicago-author-date-18th` (post wave-1) via
`report-core.js` + `analyze-parity-residuals.js --list "C year-suffix
letter"`. The 97-row count is multi-label; two clean engine roots found:

1. **Range false-positive (the literal "leaks into an adjacent date range"
   symptom).** `Disambiguator::build_group_key`
   (`crates/citum-engine/src/processor/disambiguation.rs`) keyed the
   collision on the bare *start* year (`year().parse::<i32>()`), so a
   same-author bare date and a ranged date sharing a start year (Adams
   `1930` vs. `1930/1938`) collided and got invented letters
   (`1930a`/`1930b–1938`) neither of which citeproc-js renders.
2. **Wrong letter (a↔b swap).** Genuine same-year collisions get the
   opposite letter from citeproc-js. Root: Citum's title **sort** key
   (`crates/citum-engine/src/sort_support.rs`) unconditionally strips
   leading articles ("The"); citeproc-js does not. Confirmed via 2/2
   discriminating oracle-order pairs (Fogel Technophysio/Escape, Shakespeare
   Othello/Complete Works), 0 counterexamples.

Full diagnosis, row accounting, and staged fix plan:
/home/bruce/.claude/plans/fix-bean-csl26-rrsb-on-velvety-bee.md (local plan
file; PR description carries the durable copy).

**Commit A (this PR, `codex/engine-year-suffix-range-key`) — root 1 landed:**
`build_group_key` now keys a ranged issued date on its rendered,
form-restricted text (reusing `date_slot_discriminant`) instead of the bare
start year; plain single-year dates keep the cheap integer fast path.
Probed `inline_disamb_suffix` range-suffix placement per plan — no
same-rendered-range collision exists in the corpus post-fix, so left
unchanged (no oracle evidence to act on).

Verified: `chicago-author-date-18th` 210/542 -> 211/542 (+1, 0 regressions);
`taylor-and-francis-chicago-author-date` 210/542 -> 211/542 (+1, tracks
author-date exactly, as expected — inherits by `extends:`);
`chicago-notes-18th` 30/72 -> 30/72 (0/0, no ranged-date collision in its
corpus); `chicago-shortened-notes-bibliography` 87/473 -> 87/473 (0/0).
Portfolio: `check-core-quality.js --parity-baseline
embedded-parity-baseline.json` passes (fidelity=1.0/35, exact-parity>=
baseline for all 19 embedded-core styles, no new warnings). Gate:
`cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D
warnings` + `cargo nextest run` all clean (2717/2717). 3 new unit tests in
`disambiguation.rs`.

**Root 2 (article-sort parity) is root-caused but not yet landed** — scoped
as a stacked PR (`codex/engine-title-sort-article-parity`) behind a
portfolio-wide measurement gate (disable stripping, sweep both oracles)
before deciding the fix shape (origin-gated vs. explicit schema option).

**Row accounting (don't over-promise the "97"):** most C-labeled rows carry
2+ defects owned by other waves. Commit A flips exactly
`6188419/6D3H4K8E` (sole-cause). Commit B (pending) is expected to flip
`6188419/ESET6WVE` only (`QB9KGZ82` also carries a URL/DOI label, wave 8).
NOT fixed by either commit: Davis/Hayek swaps are full-*title* ties tangled
with type-routing/edition-precedence (other waves' scope); Gourmet/OpenAI
"missing suffix" rows are tangled with author-drop/structural defects.
`Forthcoming`->`n.d.` rows are classifier noise for this bucket, not a
year-suffix defect — filed as follow-up csl26-qmxw.

## Correction + PR 2 summary (2026-08-30)

**Correction to the progress note above:** the earlier claim "(B) flips
6188419/ESET6WVE" is wrong. Root-caused further (see csl26-uy29): that
row's swap is caused by a third, distinct engine defect (same-year
issued-date sort comparing full month/day precision, so a year-only date
beats a day-precision date to the earlier letter regardless of title) --
not by article-stripping. Filed as its own bean; not fixed here.

**Commit B (`codex/engine-title-sort-article-parity`, stacked on commit
A) — two fixes landed:**

1. **Article-stripping removed** (`sort_support.rs`,
   `title_sort_key_with_options`): CSL has no automatic leading-article
   stripping: citeproc-js sorts by literal title text. Citum stripped
   unconditionally via `Locale::strip_sort_articles`, swapping year-suffix
   letters on any same-year collision where exactly one title carried a
   leading article. Confirmed via the Shakespeare pair (Othello / The
   Complete Works, no `title-short` complication): letters now correctly
   match citeproc-js order. `Locale::strip_sort_articles` and locale
   `sort_articles` data stay in place (public schema-crate API, semver);
   just unused by this call site now.
2. **`Title::Shorthand` sort-text fix** (found while investigating, not
   the Fogel explanation -- see correction above): `Title::Shorthand`'s
   `Display` composes `"short (full)"` for *rendering*; sorting by that
   composite let a short title silently override sort order. Fixed to sort
   by the full title. Currently inert on CSL-JSON-sourced fixtures (CSL-JSON's
   `title-short` isn't mapped to Citum's `short_title` field by the
   csl-legacy/citum-refs conversion path -- confirmed via `citum convert
   refs`), but a real correctness gap for any reference that does carry a
   `short_title` (e.g. native Citum YAML).

**Verified:**
- Full nextest: 2719/2719 (nextest count includes new/renamed tests; 6
  pre-existing tests encoded the old stripping behavior in their
  assertions/names and were updated to the new literal-text semantics:
  `disambiguation::year_suffix_follows_locale_collated_title_order`
  (citations.rs), `test_bibliography_per_group_disambiguation`,
  `test_sort_anonymous_work_by_title` (processor/tests.rs),
  `sorting::anonymous_works_sort_by_explicit_title_key_uses_literal_text`,
  `sorting::anonymous_titles_sort_by_literal_text`,
  `sorting::anonymous_same_year_entries_keep_years_in_order_before_tiebreaks`
  (tests/bibliography.rs -- the last rewritten to use one shared title
  across all three entries so it actually isolates year-ordering instead
  of being confounded by title-driven author-key-fallback bucketing),
  `test_apa_7th_sort_same_author_year_by_title` (tests/sort_oracle.rs).
- Full-portfolio sweep (`node scripts/report-core.js --all-features` +
  per-style `exactParity` diff against the PR-1 baseline): **0
  regressions across all 35 styles**, **+5 exact-parity**
  (`american-medical-association-alphabetical`, 22/67 -> 27/67), 34
  styles unchanged. Chicago family: +1 already landed via commit A;
  no further movement from commit B alone (Fogel doesn't flip, per the
  correction above; no other sole-cause C-labeled row in the corpus
  happens to be a clean leading-article case without a co-occurring
  defect).
- `check-core-quality.js --parity-baseline` gate: passed (fidelity=1.0/35,
  exact-parity >= floor for all 19 embedded-core styles). Regenerated
  `embedded-parity-baseline.json` from the full portfolio per
  STYLE_WORKFLOW_DECISION_RULES' shared-ancestor rule (this touches
  shared `sort_support.rs`/engine code) -- note the prior baseline was
  stale (2026-08-19), so the regenerated floors also absorb unrelated
  gains from PRs merged since then, not only this change.
- Gate: `cargo fmt --check` + `cargo clippy --all-targets --all-features
  -- -D warnings` + `cargo nextest run` all clean.

**Follow-up filed:** csl26-uy29 (date-precision tie-break defect,
confirmed root cause of the Fogel swap and structurally similar cases;
needs citeproc-js behavior research before any engine fix, portfolio-wide
blast radius since `compare_by_issued` is shared by every Issued-sort
style).

**Status:** both engine roots this bean's scope covers are now fixed and
verified (PR 1 merged pending review, PR 2 open). The Fogel-shaped
"wrong letter" residual is real but out of this bean's original two-root
scope -- tracked as csl26-uy29.
