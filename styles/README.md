# Styles Quality Metrics

`docs/compat.html` now reports two complementary metrics:

- `Fidelity`: output match rate against citeproc-js oracle.
- `Quality (SQI)`: structural quality of the CSLN style implementation.

## SQI (Style Quality Index)

SQI is a weighted score from `0` to `100`:

```
SQI =
  35% Type Coverage
  25% Fallback Robustness
  25% Concision
  15% Preset Usage
```

### 1) Type Coverage (35%)

Derived from per-type citation results in the oracle report.

- rewards high pass rate across observed reference types
- includes a breadth factor (more observed types improves score)

### 2) Fallback Robustness (25%)

Static check of bibliography fallback behavior in the base template.

For core types without explicit `type-templates`, SQI checks whether the base
template still provides usable output:

- at least one visible component
- at least two anchor components (`contributor`, `title`, or `date`)

### 3) Concision (25%)

Measures template compactness and unnecessary complexity.

Penalizes:

- excessive component count (by style-class target)
- redundant semantic components
- high override density

This keeps styles maintainable and discourages overfit templates.

### 4) Preset Usage (15%)

Rewards explicit preset reuse via `use-preset`.

- higher score for meaningful preset usage
- lower score when no presets are used and template complexity is high

## Why SQI Exists

Fidelity alone can hide implementation fragility. SQI adds signals for:

- broader type behavior
- resilience when type-specific rules do not exist
- template maintainability and readability
- reuse of style-system abstractions

Together, Fidelity + SQI provide a better compatibility and migration signal.

## Regenerating Compatibility Report

```bash
node scripts/report-core.js --write-html > /tmp/compat-report.json
```

This updates `docs/compat.html` and emits report JSON to stdout.
