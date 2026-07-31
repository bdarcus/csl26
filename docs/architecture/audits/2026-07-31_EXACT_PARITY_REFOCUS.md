# Refocusing Style/Engine Verification on Exact Parity

- **Date:** 2026-07-31
- **Bean:** `csl26-6th8` (under epic `csl26-zik7`)
- **Question:** the project's headline metric, `fidelityScore` (surfaced as
  "fidelity" / "compatibility"), is a lenient comparison that discards
  punctuation, spacing, and casing — the substance of citation formatting.
  What should replace it as the thing agents and CI actually optimize for and
  gate on, and how do we keep the gate honest when the oracle itself might be
  wrong?

## What fidelity actually measures

`scripts/oracle-utils.js` implements the lenient comparison behind
`fidelityScore`:

- `normalizeText` (`:19`) strips HTML tags, `&#38;`, markdown `_italics_` /
  `**bold**`, `[Internet]`, rewrites `sec.`→`section`, `(eds.)`/`ed.`→
  `editors`, reorders "cited"/"Accessed" date forms, maps full month names to
  3-letter abbreviations, collapses `et al.`→`et al`, deletes spaces before
  `,.;:`, collapses whitespace, and strips a leading `N.` bibliography
  numbering prefix.
- `tokenizeForSimilarity` (`:95`) then normalizes, lowercases, strips **all**
  remaining punctuation, splits on whitespace, and drops single-character
  tokens.
- `compareText` (`:172`) passes an entry if the normalized forms are equal,
  **or** if the bag-of-words Jaccard similarity of the tokenized forms is
  `>= 0.60`.

The result: word order, casing, and virtually all punctuation are invisible
to the gate. `american-medical-association` is the clearest case —
`fidelityScore: 1.0` (every entry passes), while its exact-text parity at
HEAD is 48/67 (71.6%). The report's own prose already conceded the gap
(`scripts/report-core.js`, unchanged): *"Compatibility is the existing
lenient regression gate; it can tolerate meaningful text-level drift."*

**Fidelity is retained, unchanged, as the coarse entry gate** — failing the
lenient comparison means something is structurally broken, not merely
imprecise. It stops being the metric that defines "done."

## Exact parity was already computed, just never gated

`normalizeExactText` (`oracle-utils.js:75`) strips transport markup only
(HTML tags, entity decoding, markdown emphasis, whitespace collapse) —
numbering, case, punctuation, and brackets all survive. Every oracle
comparison already carries `exactMatch` / `exactAdjudication`, and
`scripts/report-data/embedded-parity-baseline.json` already recorded a
per-style snapshot of it, with a `purpose` field stating plainly: *"the input
for future parity-improvement waves to compare against."* It carried
`gating: false` and nothing read it as a gate. The infrastructure existed;
only the policy was missing.

## Defect found and fixed: the parity aggregate ignored registered divergences

`summarizeExactParity` (`report-core.js:750`) read `oracleResult.citations` /
`oracleResult.bibliography` **raw**. Its sibling, `countCaseMismatches`
(`:734`, which feeds the fidelity-adjacent case-mismatch summary), reads via
`getEffectiveOracleSection`, which prefers `oracleResult.adjusted` — the
section where `scripts/lib/oracle-divergences.js` has already applied the
registered-divergence rules (div-004, div-005, div-008, div-009, div-010,
div-011; see `docs/adjudication/DIVERGENCE_REGISTER.md`). Those adjustments
override `match` only; they never touch `exactMatch`. So entries masked by a
registered divergence — behavior Citum deliberately chose, already excluded
from the fidelity gate — were still counted as **exact-parity failures**.

Fixed by reading through `getEffectiveOracleSection` in
`summarizeExactParity`, and by bucketing entries carrying an
`appliedDivergence` into a new `divergenceExcluded` count rather than folding
them silently into `passed` or `notComparable`. The `count_toward_fidelity:
false` scope exclusion (`verification-policy.yaml`, e.g. the GB/T
author-date/note bibliography scope) needed no change here — it already
operates upstream, in `mergeBenchmarkRunIntoOracle`, before `oracleResult`
reaches parity aggregation.

**Isolated effect of the fix**, measured by running all 19 embedded-core
styles at the identical commit (`940b461d`, `--all-features`,
`--parallelism 1` — see the determinism note under Gate design) with and
without the code change, so intervening style/engine work is not conflated
with the fix itself:

| Style | `passed` before → after | `total` before → after | `divergenceExcluded` after |
|---|---|---|---|
| `chicago-notes-18th` | 8 → 8 (unchanged) | 74 → 72 | 2 |
| `gb-t-7714-2025-note` | 240 → 240 (unchanged) | 285 → 276 | 9 |
| `gb-t-7714-2025-numeric` | 257 → 257 (unchanged) | 271 → 262 | 9 |
| all other 16 styles | unchanged | unchanged | 0 |

The fix's real, isolated footprint at this commit: **20 entries across 3
styles** move from "counted, failing" to "excluded as a registered
divergence." `passed` does not change anywhere — the fix only shrinks the
denominator for entries that were never real parity failures. `rate` rises
correspondingly for those 3 styles (e.g. `gb-t-7714-2025-numeric`
257/271 → 257/262, 94.8% → 98.1%).

Every other apparent change when comparing today's numbers against the stale
`2026-07-30` baseline or the `2026-07-28` portfolio audit — including
`american-medical-association`'s 23.9%→71.6% and
`chicago-author-date-18th`'s 19.3%→30.6% — is **not** attributable to this
fix. It reflects real style/engine work landed between those dates and
`940b461d` (the four Chicago bibliography-link commits, and whatever else
touched AMA's rendering in between). **Do not treat the pre-`940b461d`
figures — including the 24.5% portfolio number in
`2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md` and the per-style numbers in
the `2026-07-30` `embedded-parity-baseline.json` — as attributable to this
change**; they are simply out of date, for reasons unrelated to
`summarizeExactParity`.

The regenerated baseline below (portfolio total 1377/3235, 42.6%
divergence-adjusted) is still the correct thing to gate on going forward —
it is the current, correctly-measured state — but its delta from prior
snapshots should not be read as "the parity fix improved things by 4
points." Only the 20-entry `divergenceExcluded` change is the fix's
contribution; the rest is unrelated style drift, most of it improvement, one
case (below) a regression.

One observation surfaced while isolating the fix and is **out of scope for
this change**: `springer-vancouver-brackets` dropped from 28/67 to 20/67
between the `2026-07-30` baseline commit (`828cb9d2`) and `940b461d` —
confirmed unrelated to this fix (both pre-fix and post-fix code report
20/67 at `940b461d`, per the table above). This is a real regression, most
likely a side effect of the intervening Chicago bibliography-link commits.
Filed as a follow-up (see Beans below); the new baseline intentionally
records the current, lower number as the floor rather than papering over it.

## Gate design

**Scope:** all 19 embedded-core styles in
`scripts/report-data/embedded-parity-baseline.json`, matching the existing
"embedded-core = hard gate" tier rule in
`docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`.

**Mechanism — per-style monotonic floor, not a portfolio threshold:**

```
per style:  exactParity.passed >= baseline.passed   # hard fail
            exactParity.total  == baseline.total     # fixture-drift guard
```

Gating on absolute `passed` rather than `rate` matters: `rate`'s denominator
(`total`) moves whenever pairing state or fixture composition changes, so a
rate-based floor can drift without anyone touching parity at all. The
`total` equality check mirrors the shape `scripts/check-oracle-regression.js`
already uses for the top-10 monotonic gate — reused here rather than
inventing a second convention. `scripts/check-core-quality.js` implements
this via `--parity-baseline` / `--parity-adjudication`; the workflow's
`mode=selected` path (a style-YAML-only PR, `.github/workflows/fidelity.yml`)
implements the same check inline, because style-YAML PRs are exactly the
changes most likely to move parity, and that path previously only failed on
a hard error, never on a metric regression.

**Determinism.** Measured directly while validating this gate: the same
report-core invocation at the same commit produced different `exactParity`
totals across two runs at the default `--parallelism 4` (e.g. `apa-7th`
`total: 146` vs. `total: 80` on the second run), each time paired with a
`Snapshot oracle failed ... exit 2` warning for the affected styles — a
concurrent-read race against the citeproc snapshot cache that yields a
partial result without `report-core.js` treating the run as failed. Two
guards, both required:

1. `check-core-quality.js` and the `mode=selected` inline gate now treat any
   style carrying `style.error` or `style.qualityBreakdown?.error` as
   **unmeasurable** — hard-failing with its own message — before comparing
   its `exactParity` numbers at all, so a measurement gap is never reported
   as fixture drift or a false parity regression.
2. `just check-core-quality` and both `report-core.js` invocations in
   `fidelity.yml` now pin `--parallelism 1`. This is a workaround, not a fix
   for the underlying race — see the follow-up bean below — but a
   reproducible gate needs it now.

Cost of the pin, measured locally on the full default corpus (35 styles,
`--all-features`, warm build): `--parallelism 4` completed in ~28s wall,
`--parallelism 1` in ~76s wall — roughly 48s slower, not a step change for a
CI job whose Rust build already dominates. Re-measure on a colder CI runner
if this becomes noticeable; the fallback if it does is pinning parallelism
only for the gate-reading step rather than the whole report, or dropping to
`--parallelism 2`.

**Floors are set at current measurement.** This gives "start with a lower
target" for free — `chicago-shortened-notes-bibliography` floors at 2.4%
(11/465), `gb-t-7714-2025-numeric` floors at 98.1% (257/262) — while
protecting every style that is already good from silently regressing under
the new gate. **Ratcheting is: finish a tuning wave, regenerate
`embedded-parity-baseline.json`, commit the new floor.** There is no separate
threshold schedule to maintain.

A portfolio-wide number remains useful context but stays **directional
only**, in `docs/compat.html`, alongside the existing `>=95% compatibility` /
`>=90 SQI` targets — never a per-style gate.

## Escalation: what happens when the oracle looks wrong

Exact-parity residuals are high-volume (thousands of rows) and mostly
punctuation/spacing shaped — routing them through
`docs/adjudication/DIVERGENCE_REGISTER.md` would swamp a register that added
15 entries in five months of authority-classified prose. `div-010`'s own note
is the precedent for why this distinction matters: *"byte-parity against
[citeproc-js] does not catch this — the fidelity gate's proxy is weaker than
the real requirement."* Some parity mismatches are exactly the reverse case:
citeproc-js is right and Citum needs a fix.

A separate, lightweight, machine-readable ledger —
`scripts/report-data/parity-adjudication.json` — records per-entry
classifications with three states:

| State | Counts against the gate? | Who may write it |
|---|---|---|
| `citeproc-correct` | **Yes** — a required Citum fix | agent, unilaterally |
| `unclear` | No — excluded, escalates to the user | agent, unilaterally |
| `citum-correct` | No | **user only**, and only with a cited authority |

The asymmetry is deliberate: an agent can record that something is
unresolved, or that Citum is wrong, on its own judgment. An agent can never
unilaterally decide the oracle is wrong — `check-core-quality.js` rejects any
`citum-correct` entry missing an `authority` (a cited source — publisher
rules, biblatex prior art, a spec section, documentary evidence) or a
`confirmedBy` (the user). Without that asymmetry, every hard residual gets
parked in the exclusion bucket and the gate hollows out from the inside.

The gate prints the `unclear` queue size on every run (a growing queue is
visible, not free) and the ledger is seeded empty in this change — no
existing residual has been triaged yet. A `citum-correct` entry that
generalizes beyond a single style (i.e., it's really a design decision, not a
one-off) should be promoted to a full `div-NNN` register entry by the user;
the ledger is not a replacement for the register, only a staging area for
volume the register was never meant to hold.

## Metric ordering

1. **Fidelity 100%** — unchanged entry gate.
2. **Exact parity ≥ per-style floor** — new hard gate, the primary tuning
   objective for embedded-core work.
3. **SQI** — remains a hard gate for embedded-core, now ordered after parity
   in the `tune` loop (`docs/guides/STYLE_WORKFLOW_EXECUTION.md`).

This is deliberately **not** a rename. `fidelityScore`, `compatibilityScore`,
`computeFidelityScore`, `--min-fidelity`, `--skip-fidelity`, and every
baseline JSON key keep their existing names — renaming them would break
baseline comparison for no benefit. What changes is which metric agents are
told to optimize and which one CI enforces beyond the coarse entry check.

## What changed in this PR

- `scripts/report-core.js` — `summarizeExactParity` divergence-masking fix;
  `divergenceExcluded` added to the per-style and portfolio-level summaries;
  updated dashboard prose describing the gate (no more "unadjudicated,
  non-gating" claim now that per-style floors are enforced in CI).
- `scripts/report-data/embedded-parity-baseline.json` — regenerated at HEAD
  with the fixed aggregator; doubles as the new gate's baseline file.
- `scripts/report-data/parity-adjudication.json` — new, seeded empty.
- `scripts/check-core-quality.js` — `--parity-baseline` /
  `--parity-adjudication` flags; hard fail on floor regression, fixture
  drift, or an unmeasurable style (oracle/quality error); adjudication-state
  validation; unclear-queue reporting.
- `.github/workflows/fidelity.yml` — `mode=selected` now enforces the same
  fidelity/parity floors as `mode=all`, instead of only failing on a hard
  error; both paths pin `--parallelism 1` for determinism (see above).
- `justfile` — `check-core-quality` recipe wired to the new flags, pinned to
  `--parallelism 1`, and now runs with `--all-features` to match CI
  (previously the two measured different feature sets against the same
  baseline).
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md`,
  `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`, the `style-tune` /
  `style-qa` / `style-migrate-enhance` / `style-evolve` skills (both
  `.claude/skills/` and `.skills/` copies), `docs/guides/STYLE_EVOLVE_WORKFLOW.md`,
  and `.codex/agents/style-qa-reviewer.md` — updated to the three-metric
  ordering above; all defer here for the underlying rationale rather than
  restating it.

## What did not change

- The fidelity gate's mechanics, thresholds, or scope (`core-quality-baseline.json`'s
  10 styles) — untouched, per `csl26-6th8`'s own acceptance criterion: *"do
  not change lenient compatibility gates implicitly."*
- `docs/adjudication/DIVERGENCE_REGISTER.md` and its 15 existing entries.
- Dated audits under `docs/architecture/audits/**`, and
  `docs/specs/FIDELITY_RICH_INPUTS.md` / `docs/specs/CROSS_ENTRY_FIDELITY.md`
  — these record what was measured at the time; rewriting them in place of
  linking forward would falsify the historical record.

## Follow-ups (beans)

- `csl26-l5oh` — investigate the `springer-vancouver-brackets` 28→20 parity
  regression between `828cb9d2` and `940b461d` (likely a side effect of the
  Chicago bibliography-link commits).
- `csl26-7xhp` — root-cause the concurrent-read race behind the `Snapshot
  oracle failed ... exit 2` non-determinism under default `--parallelism`
  (see "Determinism" above) so the gate can drop the `--parallelism 1`
  workaround.
- Triage and classify the accumulated parity residuals per style/family into
  the new adjudication ledger (the bulk of `csl26-6th8`'s original acceptance
  criteria — clustering by semantic cause, attributing to style YAML vs.
  shared renderer vs. intentional divergence — remains open work, now with a
  concrete gate and ledger to record the results in).
