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
updated_at: 2026-08-30T13:29:17Z
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
