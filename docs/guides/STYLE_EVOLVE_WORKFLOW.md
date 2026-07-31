# Style Evolve Workflow

## Who This Is For
New contributors who want one command path for style work.

## One Command Rule
Use `/style-evolve` for all style tasks.

Do not choose internal skills directly unless you are maintaining the workflow.

For Codex CLI use, install the repo-owned skills with
`./scripts/install-skills.sh` and invoke the named skills directly.
`./scripts/codex <role> <target...>` remains available as a fallback for direct role
targeting.

## Modes
1. `upgrade`
- Improve existing Citum style(s).
- Typical goal: increase SQI while preserving or improving fidelity.

2. `migrate`
- Convert one or more CSL 1.0 styles into Citum styles.
- Typical goal: high oracle parity plus maintainable templates.
- For embedded-core targets, the output is a **seed** for a `tune` pass, not a
  finished embedded style.

3. `create`
- Create a style from reference evidence.
- Accepts mixed source inputs:
  - `--source-url`
  - `--source-text`
  - `--source-issue`
  - `--source-file`

4. `tune`
- Drive an **embedded-core** style to 100% fidelity, a raised exact-parity
  floor, **and** clean SQI via iterative LLM authoring, seeded by migrate
  evidence.
- Fidelity, exact parity, and SQI are all hard gates, applied in that order.
  Use only for styles in `citum style list --source embedded`. See
  [2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md)
  for why fidelity alone is not sufficient evidence of correct rendering.

## Examples
```bash
/style-evolve upgrade --styles styles/elsevier-harvard.yaml --target-sqi 0.90
/style-evolve migrate --legacy styles-legacy/apa.csl --count 1
/style-evolve create --source-url https://example-style-guide.org --source-text "example citations and bibliography"
/style-evolve tune ieee
```

## Quality Policy
- Fidelity to the declared primary authority is the hard gate for all styles.
  It tolerates meaningful text-level drift (punctuation, casing, spacing) by
  design — it is a structural entry gate, not proof of correct rendering.
- **Exact parity is a hard gate for embedded-core styles.** A style's
  exact-parity `passed` count may never drop below the floor recorded in
  `scripts/report-data/embedded-parity-baseline.json`; it is the primary
  tuning objective. Diagnostic-only for dependent/long-tail styles.
- **SQI is a hard gate for embedded-core styles**, ordered after fidelity and
  exact parity. Fidelity, exact parity, and clean SQI are all required for
  any embedded-core style pass. For dependent/long-tail styles, SQI is a
  secondary optimization metric.
- For styles with configured `benchmark_runs`, official rich-input evidence is auto-run as supplemental advisory output.
- Every iteration must assess both:
  - style-level edits
  - processor/preset/feature opportunities
- Before editing YAML, classify the target using the shared three-axis taxonomy:
  semantic class (`base`, `profile`, `journal`, `independent`),
  implementation form (`alias`, `config-wrapper`, `structural-wrapper`, `standalone`),
  and portfolio tier (`embedded-core` or `dependent`).
- Profile work must preserve the config-wrapper contract: scoped options and
  metadata only, with no local templates, no local `type-variants`, and no
  template-clearing `null`.
- Journal descendants may legitimately remain structural wrappers when
  guide-backed deltas or current merge mechanics prevent a meaningful thin
  reduction.
- Choose parent styles from current guide-backed authority first, not nearest
  CSL/template similarity.
- If guide-backed parentage is real but the current merge model still forces a
  bulky child file, record the infrastructure constraint and stop forcing
  compression.
- True profiles may use documentary-primary verification when the verification
  policy says so.

## Wave Guidance

A style wave is a bounded cohort executed through repeated `upgrade`, `migrate`,
`create`, or `tune` passes under the shared workflow docs.

- Keep one wave to one family or one clearly related cohort per PR.
- Profile-family work may require a `create` pass for a hidden family root,
  followed by `upgrade` passes to reduce the public handles.
- A wave is execution scope, not a separate mode.

## Internal Pipeline (For Maintainers)
`/style-evolve` routes internally to:
- `style-maintain` (upgrade path)
- `style-migrate-enhance` (migrate path)
- `style-tune` (tune path — Claude Code host)
- `style-qa` (required gate — tier-aware)
- `pr-workflow-fast` (branch/PR efficiency)

`/styleauthor` remains available as a legacy alias and forwards to `/style-evolve`.

For Codex CLI users outside the host slash-command UI, use the named skills
first:

```bash
./scripts/install-skills.sh
# Then invoke $style-evolve, $migrate-research, or $rust-simplify in Codex.
```

Use `./scripts/codex <role> <target...>` only when you need direct access to an
internal role contract.
