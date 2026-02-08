# Regression Detection Baselines

This directory stores baseline test results for regression detection in the rendering fidelity workflow.

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
    "duration": "45.2s"
  }
}
```

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

See [docs/RENDERING_WORKFLOW.md](../docs/RENDERING_WORKFLOW.md) and [docs/WORKFLOW_ANALYSIS.md](../docs/WORKFLOW_ANALYSIS.md) for how regression detection integrates into the overall rendering fidelity workflow.
