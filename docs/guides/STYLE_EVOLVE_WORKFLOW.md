# Style Evolve Workflow

## Who This Is For
New contributors who want one command path for style work.

## One Command Rule
Use `/style-evolve` for all style tasks.

Do not choose internal skills directly unless you are maintaining the workflow.

## Modes
1. `upgrade`
- Improve existing CSLN style(s).
- Typical goal: increase SQI while preserving or improving fidelity.

2. `migrate`
- Convert one or more CSL 1.0 styles into high-quality CSLN styles.
- Typical goal: high oracle parity plus maintainable templates.

3. `create`
- Create a style from reference evidence.
- Accepts mixed source inputs:
  - `--source-url`
  - `--source-text`
  - `--source-issue`
  - `--source-file`

## Examples
```bash
/style-evolve upgrade --styles styles/elsevier-harvard.yaml --target-sqi 0.90
/style-evolve migrate --legacy styles-legacy/apa.csl --count 1
/style-evolve create --source-url https://example-style-guide.org --source-text "example citations and bibliography"
```

## Quality Policy
- Fidelity is the hard gate.
- SQI is a secondary optimization metric.
- Every iteration must assess both:
  - style-level edits
  - processor/preset/feature opportunities

## Internal Pipeline (For Maintainers)
`/style-evolve` routes internally to:
- `style-maintain` (upgrade path)
- `style-migrate-enhance` (migrate path)
- `style-qa` (required gate)
- `pr-workflow-fast` (branch/PR efficiency)

`/styleauthor` remains available as a legacy alias and forwards to `/style-evolve`.
