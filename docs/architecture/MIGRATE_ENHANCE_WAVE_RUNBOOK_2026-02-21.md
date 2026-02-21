# Migrate+Enhance Wave Runbook (2026-02-21)

## Purpose
Single handoff and execution document for the current wave process.
Use this as the canonical source for status, metrics, and next actions.

## Scope
- Branch: `codex/migrate-enhance-wave-strategy`
- Draft PR: <https://github.com/bdarcus/csl26/pull/208>
- Primary goal: improve `csln-migrate` fidelity/SQI through wave-based style
  conversion, then promote repeated fixes into shared migrate/processor logic.

## Current Results

### Wave 1 (note-heavy, 12 styles)
- Baseline: `619/664` combined (citations `385/408`, bibliography `234/256`)
- Current: `642/664` combined (citations `408/408`, bibliography `234/256`)

### Wave 2 (numeric variants, 12 styles)
- Baseline: `450/528` combined (citations `76/144`, bibliography `374/384`)
- Script-level checkpoint: `514/528` (citations `140/144`)
- Rust/processor checkpoint: `518/528` (citations `144/144`)

Wave 2 citation status is now fully closed (`144/144`).

### Wave 3 (author-date + author/label diversity, 12 styles)
- Baseline: `458/541` combined (citations `114/156`, bibliography `344/385`)
- Dominant citation mismatch clusters:
  - `suppress-author-with-locator` (9)
  - `et-al-with-locator` (9)
  - `et-al-single-long-list` (9)
  - `disambiguate-add-names-et-al` (9)

## Landed Enhancements

### Merge workflow (`scripts`)
- `scripts/merge-migration.js`
  - prevent empty inferred templates from clobbering non-empty base templates
  - numeric citation fallback for explicit empty citation templates
  - numeric locator normalization for AMA-like patterns

### Migration (`csln-migrate`)
- `crates/csln_migrate/src/options_extractor/bibliography.rs`
  - extract legacy bibliography sort into CSLN `GroupSort`
- `crates/csln_migrate/src/main.rs`
  - emit extracted sort into generated `bibliography.sort`
- `crates/csln_migrate/src/options_extractor/tests.rs`
  - coverage for new bibliography sort extraction

### Processor sorting (`csln-processor`)
- `crates/csln_processor/src/grouping/sorting.rs`
  - context-aware author-key fallback behavior
  - author->title fallback only when sort template includes `title`
  - missing-name entries sort after named entries when no title key exists

## Remaining Gaps
- Wave 2 bibliography remains `374/384` (10 unmatched entries).
- Wave 3 migration/processor promotion pass and rerun are pending.

## Next Execution Slice
1. Apply migrate/processor promotion only for repeated (2+) Wave 3 mismatch
   patterns.
2. Re-run Wave 3 and record baseline vs post-enhancement deltas.
3. Re-check core quality drift:
   - `node scripts/report-core.js > /tmp/core-report.json`
   - `node scripts/check-core-quality.js --report /tmp/core-report.json --baseline scripts/report-data/core-quality-baseline.json`

## Bean Link
Tracked in bean: `csl26-w2n8`.

## Related Docs
- `docs/architecture/MIGRATE_ENHANCE_WAVE_STRATEGY_2026-02-21.md`
- `docs/architecture/MIGRATE_ENHANCE_WAVE1_HANDOFF_2026-02-21.md`
- `docs/architecture/MIGRATE_ENHANCE_WAVE2_HANDOFF_2026-02-21.md`
- `docs/architecture/MIGRATE_ENHANCE_WAVE3_HANDOFF_2026-02-21.md`
