# Styles Quality Metrics

This directory now contains Citum's **exemplar** tier: 17 styles retained as
Rust-test fixtures, embedded-parent wrapper examples, or unique behavior
coverage (16 appear in `docs/compat.html`; `alpha` is excluded there — see
`SKIPPED_STYLES` in `scripts/report-core.js` — because it is biblatex-derived
with no citeproc-js counterpart and no snapshot). The compiled **embedded**
tier is authoritative at
`crates/citum-schema-style/embedded/styles/` (also exposed here through the
`embedded/` symlink). The remaining community corpus lives in
[`citum/citum-styles`](https://github.com/citum/citum-styles); it may extend
embedded parents through the registry filesystem layer.

`styles/experimental/` remains outside these tier rules.

| Tier | Location | Purpose |
|---|---|---|
| Embedded | `embedded/` | Compiled product surface, tuned toward exact-text parity. |
| Exemplar | this directory | Fixtures, wrapper examples, and unique behavior coverage. |
| Community | [`citum-styles`](https://github.com/citum/citum-styles) | Runtime-resolved extensions with advisory parity. |

The initial disposition is recorded in [`scripts/report-data/style-disposition-2026-07-28.tsv`](../scripts/report-data/style-disposition-2026-07-28.tsv); the governing policy is [`STYLE_INHERITANCE.md`](../docs/specs/STYLE_INHERITANCE.md).

`docs/compat.html` now reports two complementary metrics:

- `Fidelity`: output match rate against citeproc-js oracle.
- `Quality (SQI)`: structural quality of the Citum style implementation.

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

- excessive component count after accounting for legitimate family breadth
- duplicated `type-variants` and `type-templates`
- near-duplicate template forks that differ only slightly
- repeated copied component/group patterns across scopes
- high override density

This keeps styles maintainable and discourages overfit templates.

### 4) Preset Usage (15%)

Rewards explicit preset reuse via `extends`.

- higher score for meaningful preset usage
- lower score when no presets are used and template complexity is high
- wrapper styles can still score well when they defer real structure to shared presets

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
