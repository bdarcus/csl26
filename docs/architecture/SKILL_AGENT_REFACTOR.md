# Skill and Agent Refactor (2026-02-21)

## Objective
Improve execution speed and reduce instruction drift by separating routing,
planning, implementation, QA, and PR workflow concerns.

## Problems Addressed
- Overloaded style workflow mixed routing, planning, build, and QA.
- Agent contracts overlapped and conflicted (planner code expectations vs
  no-code policy, planner verification requirements without shell tools).
- PR workflow expectations were implicit instead of codified.

## New Skill Topology
- `style-evolve` (router): human-facing entrypoint that routes tasks.
- `styleauthor` (legacy alias): forwards to `style-evolve`.
- `style-maintain`: single-style maintenance and focused fixes.
- `style-migrate-enhance`: batch migration waves with before/after/rerun
  metrics.
- `style-qa`: standardized fidelity/SQI/format gate.
- `pr-workflow-fast`: branch/PR process with change-type validation gates.

## Agent Role Purity
- `@dstyleplan`: deep research and architecture only.
- `@styleplan`: implementation planning and escalation boundaries only.
- `@styleauthor`: execution and verification only.

## PR Efficiency Pattern
1. Create narrow `codex/*` branch.
2. Apply minimal diff needed to pass objective and gates.
3. Run checks based on touched change type.
4. Open PR with concise evidence: scope, validation, risks, follow-ups.

## Expected Outcomes
- Lower token and iteration cost for common style tasks.
- Fewer planner/implementer handoff ambiguities.
- Faster, more consistent PRs with explicit quality evidence.
