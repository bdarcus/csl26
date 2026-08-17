# Regression Detection Baselines

This directory stores local baseline snapshots for regression detection in the
rendering fidelity workflow.

These files are distinct from the canonical committed CI baselines:

- `scripts/report-data/oracle-top10-baseline.json`
- `scripts/report-data/core-quality-baseline.json`

Use `baselines/` for local milestone snapshots, refactor safety checks, and
ad hoc comparison runs. Do not treat it as the source of truth for CI.

## Usage

### Save a Baseline

After reaching a milestone (e.g., completing a tier or major feature):

```bash
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --save baselines/baseline-$(date +%F).json
```

### Compare Against Baseline

When testing changes to detect regressions:

```bash
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --compare baselines/baseline-2026-02-06.json
```

### Output Example

```
=== REGRESSION ANALYSIS ===

⚠️  REGRESSIONS DETECTED: 1 styles
  - apa:
      Bibliography: 5/5 → 4/5 (-1)

🎉 IMPROVEMENTS: 2 styles
  + ieee:
      Citations: 12/15 → 15/15 (+3)
  + nature:
      Bibliography: 0/15 → 5/15 (+5)

NET IMPACT:
  Citations: +3 passing entries
  Bibliography: +4 passing entries
  Unchanged: 17 styles
```

## Baseline File Format

Baseline files are JSON with the structure:

```json
{
  "totalStyles": 20,
  "citationsPerfect": 15,
  "bibliographyPerfect": 8,
  "componentIssues": {
    "year:formatting": 5,
    "volume:missing": 3
  },
  "styleBreakdown": [
    {
      "style": "apa",
      "citations": "15/15",
      "bibliography": "5/15",
      "citationsPct": 100,
      "bibliographyPct": 33
    }
  ],
  "metadata": {
    "timestamp": "2026-02-06T12:00:00.000Z",
    "gitCommit": "abcdef0",
    "generator": "scripts/oracle-batch-aggregate.js",
    "fixture": "tests/fixtures/citations-expanded.json",
    "styleSelector": "top:20",
    "styles": ["apa", "ieee"],
    "duration": "45.2s"
  }
}
```

Committed baseline/report artifacts should always carry:

- `metadata.timestamp`
- `metadata.gitCommit`
- `metadata.fixture`
- `metadata.generator`
- `metadata.styles` or `metadata.styleSelector`

## Best Practices

### When to Save Baselines

- **After completing a tier** (Tier 1, Tier 2, etc.)
- **Before major refactoring** (to detect regressions during refactor)
- **After fixing major issues** (to track progress)
- **Before releases** (to ensure no regressions)

### Baseline Naming Convention

Use ISO date format for easy sorting:

- `baseline-2026-02-06.json` - Date-based baseline
- `baseline-tier1-complete.json` - Milestone-based baseline
- `baseline-pre-refactor.json` - Pre-change baseline

### Retention Policy

- Keep milestone baselines indefinitely
- Keep daily baselines for 1 week
- Archive old baselines to `baselines/archive/` if needed

## Git Ignore

Baseline JSON files are gitignored by default (`baselines/*.json`) to avoid repository bloat. Only commit significant milestone baselines if needed for reference.

## Integration with Workflow

See [docs/guides/RENDERING_WORKFLOW.md](../docs/guides/RENDERING_WORKFLOW.md) and [docs/guides/WORKFLOW_ANALYSIS.md](../docs/guides/WORKFLOW_ANALYSIS.md) for how regression detection integrates into the overall rendering fidelity workflow.

## CI Canonical Baselines

The committed CI baselines live at:

- `scripts/report-data/oracle-top10-baseline.json`
- `scripts/report-data/core-quality-baseline.json`

CI checks these files via:

```bash
node scripts/check-testing-infra.js

node scripts/check-oracle-regression.js \
  --baseline scripts/report-data/oracle-top10-baseline.json

node scripts/report-core.js > /tmp/core-report.json
node scripts/check-core-quality.js \
  --report /tmp/core-report.json \
  --baseline scripts/report-data/core-quality-baseline.json
```

Refresh committed baselines only in dedicated baseline PRs with a short
before/after summary and justification for the reset.

## Fixture-Change Fan-Out

Editing `tests/fixtures/references-expanded.json` or
`tests/fixtures/citations-expanded.json` (the corpus-wide defaults) fans out
into oracle snapshots, registered coverage-audit manifests, and — only when
the change is an intended, reviewed baseline reset — the two ratcheted floors
above. `just fixture-refresh` is the single entrypoint for that fan-out (run
`just fixture-refresh yes` to also touch the ratcheted floors); see
[docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md](../docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md)
for the full map and why a scoped fixture family
(`scripts/lib/style-verification.js`'s `FIXTURE_SET_REFS`) is usually the
better target for a new regression fixture than the shared default.
