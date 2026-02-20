# Priority Batch Migrate+Enhance Checklist

## Batch Scope
- [ ] Requested batch size (`N`):
- [ ] Source priority list reviewed (`docs/reference/STYLE_PRIORITY.md`)
- [ ] Current status reviewed (`docs/TIER_STATUS.md`)
- [ ] Selected styles listed:

## Baseline (Auto-Migrated)
- [ ] `csln-migrate` run for each selected style
- [ ] Baseline report captured (`node scripts/report-core.js`)
- [ ] Baseline metrics recorded per style:
  - [ ] Fidelity
  - [ ] SQI
  - [ ] Citation pass count
  - [ ] Bibliography pass count

## Enhancement Loop (Styleauthor)
- [ ] Fidelity issues fixed first
- [ ] SQI improvements applied without fidelity regression
- [ ] Structural outliers isolated with minimal `type-templates`
- [ ] Oracle checks completed for all selected styles

## Rerun Comparison (Required)
- [ ] `csln-migrate` rerun on the same selected styles
- [ ] Edited vs rerun comparison table completed:
  - [ ] Fidelity delta
  - [ ] SQI delta
  - [ ] Citation delta
  - [ ] Bibliography delta

## Migration Pattern Extraction
- [ ] Repeated gaps identified across 2+ styles
- [ ] Candidate `csln-migrate` refinements listed
- [ ] Candidate presets extracted (if applicable)
- [ ] Core-style regression check completed

## Delivery
- [ ] Final metrics table included (baseline, edited, rerun)
- [ ] Summary of migration code impact included
- [ ] Follow-up recommendations listed
