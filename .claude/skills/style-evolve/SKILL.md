# Style Evolve

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Single human-facing command for CSLN style co-evolution (style + processor) with fidelity as the hard gate and SQI as secondary optimization.

## Human UX (Public Entry Point)
Use this command for all style work:
- `/style-evolve upgrade ...`
- `/style-evolve migrate ...`
- `/style-evolve create ...`

Do not require users to choose internal pipeline skills.

## Modes
1. `upgrade`
- Improve existing CSLN styles.
- Route to `../style-maintain/SKILL.md`.

2. `migrate`
- Convert one or more CSL 1.0 styles to high-fidelity CSLN.
- Route to `../style-migrate-enhance/SKILL.md`.

3. `create`
- Build a style from source evidence.
- Accept one or more sources:
  - `--source-url`
  - `--source-text`
  - `--source-issue`
  - `--source-file`
- Default planner path: `@dstyleplan` -> `@styleplan` -> `@styleauthor`.

## Co-Evolution Rule (Mandatory)
Every iteration must assess two tracks:
- Track A: style/template edits
- Track B: reusable code opportunities (presets, missing features, processor fixes)

A task is not complete until Track B is explicitly marked as:
- implemented, or
- deferred with rationale.

## Shared Gates
- Fidelity regression is never allowed.
- SQI is optimization only after fidelity.
- All modes must pass `../style-qa/SKILL.md` before completion.

## Output Contract
- Fidelity metrics (citations and bibliography pass counts).
- SQI delta.
- Code-opportunity table:
  - preset candidate
  - missing feature
  - processor defect
  - defer rationale

## Internal Skills (Pipeline Components)
- `style-maintain`
- `style-migrate-enhance`
- `style-qa`
- `pr-workflow-fast`

## Backward Compatibility
`/styleauthor` remains available as a legacy alias and forwards to this skill.
