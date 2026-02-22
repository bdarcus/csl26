# Style QA Gate

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Standardized verification and release decision for style work.

## Gate Inputs
- Style path(s) changed.
- Oracle JSON result(s).
- Optional baseline metrics for comparison.

## Required Checks
1. Fidelity summary:
   - citations passed/total
   - bibliography passed/total
2. SQI drift summary (secondary metric only).
3. Formatting audit:
   - double spaces
   - spaces before punctuation (` :`, ` ,`, ` .`)
   - delimiter collisions from prefix/suffix + group delimiters
4. Regression surface:
   - impacted style family/priority rank
   - likely cross-style risk

## Decision Rules
- Reject when fidelity regresses.
- Reject when formatting defects are introduced.
- Approve when fidelity is preserved or improved and formatting is clean.

## Standard Output
- Verdict: `approve` or `reject`
- Metrics line: citations + bibliography + SQI delta
- Findings: short numbered list
- Next step: merge, iterate, or escalate to planner/processor

## Suggested Commands
- `node scripts/oracle.js <legacy-style> --json`
- `node scripts/report-core.js`
- `./scripts/lint-rendering.sh <style-path>`
