# Styleauthor Workflow Efficiency Review (APA 7th Fidelity)

Date: 2026-02-18

## Outcome

- Target met for this run: bibliography fidelity moved above 20 matches (`21/27`).
- Citation fidelity remained `26/28`.

## What Worked

- Iterative oracle-driven edits found high-yield fixes quickly once mismatches were grouped by type.
- Type-specific `bibliography.type-templates` provided better precision than broad global overrides.
- Running full Rust validation gate after processor changes prevented regressions.

## Main Efficiency Losses

- Oracle mismatch extraction was repeated with ad-hoc `jq` commands.
- Several passes were spent on citation-side edge cases while bibliography was the stated priority.
- Some reference-type normalization gaps (for legacy aliases) caused style overrides to miss, increasing retries.

## Suggested Workflow Changes

1. Add a “targeted fidelity mode” path in the skill:
   - Require user goal metric at start (for example: `bibliography >20`).
   - Restrict each loop to goal-relevant mismatches only.
2. Standardize a single mismatch extraction command:
   - Persist oracle JSON once per loop.
   - Print only unmatched entries in a compact normalized format.
3. Use a two-pass prioritization rule:
   - Pass A: high-yield type-template fixes.
   - Pass B: processor/type-conversion fixes only if style-only pass stalls.
4. Add an iteration cap for non-goal dimensions:
   - Example: if bibliography goal is active, cap citation-side work unless it blocks bibliography.
5. Add explicit “commit boundary” guidance:
   - Commit immediately when target metric is reached, before optional polish.

## Proposed KPI Tracking

- Iterations to target.
- Oracle runs to target.
- Full validation gate runs.
- Final metric delta from start to finish.

