---
name: style-qa
type: agent-invocable
description: Standardized QA gate for style work. Verifies fidelity, exact-parity drift, SQI drift, formatting defects, and regression surface. Produces approve/reject verdict with numbered findings.
model: haiku
---

# Style QA Gate

Authoritative shared process docs:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md`
- `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md`

## Gate Inputs
- Style path(s) changed.
- Portfolio tier: `embedded-core` or `dependent` (from `citum style list --source embedded`).
- Oracle JSON result(s).
- Exact-parity numbers from `node scripts/report-core.js --style <name> --all-features`
  (`exactParity.passed`/`.total`) compared against
  `scripts/report-data/embedded-parity-baseline.json`, for `embedded-core` styles.
- Optional SQI report from `node scripts/report-core.js --style <name>`.
- Optional baseline metrics for comparison.
- Optional `scripts/report-data/parity-adjudication.json` diff when the pass
  recorded new adjudication entries.
- Optional docs/beans diff when task updates `.md` or `.beans/*`.
- Coverage-audit status (`current`, `stale`, or `not registered`) and, when
  registered, the regenerated packet plus disposition and joined-parity deltas.

## Required Checks
1. Fidelity summary.
2. Exact-parity summary for `embedded-core` styles — tier-weighted (see Decision Rules).
3. SQI summary — tier-weighted (see Decision Rules).
4. Formatting audit.
5. Regression surface.
6. Adjudication ledger hygiene — any new `citum-correct` entry must carry
   `authority` and `confirmedBy`; reject if either is missing (the CI gate
   also enforces this, but QA should catch it first).
7. Docs/beans hygiene when docs or beans are touched.
8. Coverage-audit freshness — reject a registered packet unless
   `node scripts/check-style-coverage-audits.js --status <style-id>` reports
   `current`. `Not registered` is valid and does not require packet creation.

## Decision Rules
- Reject when fidelity regresses — applies to all tiers.
- **Reject when exact-parity `passed` regresses for `embedded-core` styles.**
  Exact parity is a hard gate, and the primary tuning objective, for the
  embedded portfolio — see the tier table in Decision Rules.
- For `dependent` styles, exact-parity drift is diagnostic only; do not
  reject on it alone.
- **Reject when SQI is not clean for `embedded-core` styles.** SQI is a hard
  gate alongside fidelity and exact parity for the embedded portfolio.
- For `dependent` styles, SQI drift is advisory only; do not reject on SQI alone.
- Reject when a registered divergence is reported as an unexplained defect.
- Reject when a residual is classified `processor-defect` (conversion)
  without the conversion-layer pre-flight evidence required by the
  Decision Rules.
- Reject when an agent-authored `parity-adjudication.json` entry uses state
  `citum-correct` — that state is user-only.
- Reject when formatting defects are introduced.
- Reject stale or invalid registered packets. Treat explained count movement as
  evidence, not an automatic failure; uncovered fields remain investigation
  leads rather than proven causes.
- Approve when fidelity is preserved or improved, exact parity is preserved
  or improved (for `embedded-core`), formatting is clean, and (for
  `embedded-core`) SQI is clean.

## Standard Output
- Verdict: `approve` or `reject`
- Tier: `embedded-core` or `dependent`
- Metrics line: citations + bibliography + exact-parity passed/total (embedded-core
  only) + SQI score (and delta from baseline)
- Coverage-audit line: `current`, `stale`, or `not registered`, plus
  disposition and joined-parity deltas when registered
- Findings: short numbered list
- Next step: merge, iterate, or escalate to planner/processor
