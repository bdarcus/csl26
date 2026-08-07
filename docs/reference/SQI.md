# Style Quality Index (SQI)

SQI is a secondary quality metric for Citum styles.

Use SQI to improve style maintainability only after correctness is settled.

## Priority Order

1. **Exact parity** (headline correctness signal): byte-level, symmetric
   comparison of rendered text against the style's oracle. This is what
   `report-core.js`'s `exactParityOverall` measures and what
   `check-core-quality.js` gates per style — a style's exact-parity `passed`
   count may never drop below its recorded baseline floor.
2. **Fidelity** (coarse regression tripwire, retained for historical/legacy
   styles): a normalized-text pass/fail rate over the same comparisons —
   strictly weaker than exact parity and carrying no information it doesn't
   already carry. For styles listed in `core-quality-baseline.json`, fidelity
   must additionally stay at exactly `1.0` (a hard gate); for styles outside
   that baseline, including the Chicago family, exact parity alone is the
   tuning target — see
   [`CHICAGO_FAMILY_STRATEGY.md`](../specs/CHICAGO_FAMILY_STRATEGY.md) for the
   reasoning.
3. SQI (secondary): choose cleaner, more robust style definitions when
   correctness is comparable.

Never accept an SQI gain that causes an exact-parity or fidelity regression.

SQI is not the structural lint. Deterministic style-shape rules — anonymous-anchor
rejection, dead-config detection, and localization integrity (`STYLE010`:
hardcoded prose that duplicates an existing locale term, see
[`LOCALIZATION_INTEGRITY.md`](../policies/LOCALIZATION_INTEGRITY.md)) — are
enforced separately by `scripts/style-structure-lint.js`.

## What SQI Measures

SQI is computed per style from four subscores:

1. `typeCoverage`: how broadly the style succeeds across observed reference types.
2. `fallbackRobustness`: whether core types still render correctly via shared templates/fallback paths.
3. `concision`: measures how efficiently the style achieves its rendering goals through template reuse.
   - Scores authored style structure. Thin root `extends:` wrappers are scored as inherited preset use instead of being charged for resolved parent complexity.
   - Counts full authored template-bearing scopes, including full `type-variants` and `type-templates`.
   - Reports diff-form `type-variants` as patch operations. They still count selector breadth, but do not create duplicate, near-duplicate, or repeated-pattern penalties.
   - Penalizes high variant-selector counts, exact duplicate scopes, near-duplicate scopes, and repeated copied component/group patterns across full template scopes.
   - Uses structural fingerprints of whole components and groups rather than coarse field-name matching, so copied template forks are visible to the metric.
4. `presetUsage`: reuse of shared presets (`processing`, `contributors`, `dates`, `titles`, `substitute`, template presets). Root `extends:` is treated as strong embedded preset reuse when the authored wrapper has no local template scopes.

Overall SQI is reported as a 0.0-1.0 score in JSON and as a percentage in `docs/compat.html`.

`qualityBreakdown.subscores.concision` now includes supporting diagnostics such as scope count, variant count, exact duplicates, near-duplicates, repeated-pattern totals, inherited preset ID, diff variant scope count, and diff operation count so score changes are explainable.

## Working Thresholds

Current wave target, as a mean across the **embedded tier only**
(`docs/compat.html`'s headline; exemplar and community styles are reported
separately and are not held to this target):

- `>= 0.95` mean fidelity (reported as "Compatibility" in `docs/compat.html`'s
  per-style table for historical reasons; see the priority order above)
- `>= 0.90` mean SQI

These are directional wave-planning targets, not a per-style gate and not a
replacement for oracle checks. The actual enforced gate is
`scripts/check-core-quality.js`: a per-style exact-parity floor (never drop
below the recorded `passed` count in `scripts/report-data/embedded-parity-baseline.json`)
plus, for styles listed in `scripts/report-data/core-quality-baseline.json`,
a hard `fidelityScore === 1.0` requirement. SQI drift (`concision`,
`presetUsage` subscores) is checked against the same baseline but is
warn-only, not gating.

## Commands

Generate the core report:

```bash
node scripts/report-core.js > /tmp/core-report.json
```

Regenerate the compatibility dashboard:

```bash
node scripts/report-core.js --write-html
```

Check drift against CI baseline:

```bash
node scripts/check-core-quality.js \
  --report /tmp/core-report.json \
  --baseline scripts/report-data/core-quality-baseline.json
```

## Related

- [SQI refinement plan](../policies/SQI_REFINEMENT_PLAN.md)
- [SQI integrity audit](../architecture/2026-05-07_SQI_INTEGRITY_AUDIT.md)
- [Rendering workflow](../guides/RENDERING_WORKFLOW.md)
- [Style author guide](../guides/style-author-guide.md)
