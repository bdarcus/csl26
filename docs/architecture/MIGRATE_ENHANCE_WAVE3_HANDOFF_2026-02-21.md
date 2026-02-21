# Wave 3 Handoff: Author-Date + Label/Author Diversity (2026-02-21)

> Canonical status/next-actions now live in:
> `docs/architecture/MIGRATE_ENHANCE_WAVE_RUNBOOK_2026-02-21.md`

## Branch
`codex/csl26-w2n8-wave3-quickwins`

## Scope
Wave 3 from
`docs/architecture/MIGRATE_ENHANCE_WAVE_STRATEGY_2026-02-21.md`.

Goal: establish the baseline for the 12 planned author-date/label-diversity
parents and identify repeated migration mismatch patterns for promotion into
`csln-migrate`/processor logic.

## What Was Completed
1. Generated all 12 Wave 3 styles via:
`./scripts/prep-migration.sh styles-legacy/<style>.csl --agent`
2. Captured baseline oracle metrics per style via:
`node scripts/oracle.js styles-legacy/<style>.csl --json`
3. Aggregated citation mismatch IDs to identify repeated (2+) clusters.

## Batch Styles (12)
- `styles/american-fisheries-society.yaml`
- `styles/american-statistical-association.yaml`
- `styles/american-marketing-association.yaml`
- `styles/sage-harvard.yaml`
- `styles/harvard-cite-them-right.yaml`
- `styles/nlm-name-year.yaml`
- `styles/new-harts-rules-author-date-space-publisher.yaml`
- `styles/springer-basic-author-date-no-et-al.yaml`
- `styles/springer-physics-author-date.yaml`
- `styles/chicago-author-date-classic.yaml`
- `styles/the-company-of-biologists.yaml`
- `styles/modern-language-association.yaml`

## Baseline Oracle Results

| Style | Citations | Bibliography |
|---|---:|---:|
| american-fisheries-society | 9/13 | 30/32 |
| american-statistical-association | 9/13 | 32/32 |
| american-marketing-association | 9/13 | 29/32 |
| sage-harvard | 11/13 | 31/32 |
| harvard-cite-them-right | 9/13 | 32/32 |
| nlm-name-year | 8/13 | 32/32 |
| new-harts-rules-author-date-space-publisher | 9/13 | 28/32 |
| springer-basic-author-date-no-et-al | 10/13 | 30/32 |
| springer-physics-author-date | 10/13 | 10/33 |
| chicago-author-date-classic | 10/13 | 30/32 |
| the-company-of-biologists | 9/13 | 32/32 |
| modern-language-association | 11/13 | 28/32 |

Wave aggregate baseline:
- Citations: `114/156` (73.1%)
- Bibliography: `344/385` (89.4%)
- Combined: `458/541` (84.7%)

## Citation Mismatch Clusters (Baseline)
Repeated across 2+ styles:
1. `suppress-author-with-locator` (9 styles)
2. `et-al-with-locator` (9 styles)
3. `et-al-single-long-list` (9 styles)
4. `disambiguate-add-names-et-al` (9 styles)
5. `with-locator` (3 styles)
6. `locator-section-with-suffix` (2 styles)

Single-style miss:
- `single-with-prefix-and-suffix` (1 style)

## Immediate Promotion Candidate
All 12 baseline outputs currently omit `options.processing`, which forces
processor defaults instead of style-specific disambiguation settings. This is
the first repeated migrate gap to promote in Rust for Wave 3 rerun.

## Post-Promotion Checkpoint (`csln-migrate`)

Promotion applied:
- `crates/csln_migrate/src/options_extractor/processing.rs`
  - recurse into citation macro trees when detecting author-date processing
  - default extracted disambiguation to:
    - `names: false`
    - `add-givenname: false`
    - `year-suffix: true`

Wave rerun aggregate after promotion:
- Citations: `129/156` (82.7%) `(+15)`
- Bibliography: `344/385` (89.4%) `(no regression)`
- Combined: `473/541` (87.4%) `(+15)`

Per-style citation deltas:
- `american-fisheries-society`: `9/13` -> `11/13`
- `american-statistical-association`: `9/13` -> `11/13`
- `nlm-name-year`: `8/13` -> `10/13`
- `springer-basic-author-date-no-et-al`: `10/13` -> `13/13`
- `springer-physics-author-date`: `10/13` -> `13/13`
- `the-company-of-biologists`: `9/13` -> `12/13`

Citation mismatch clusters after promotion:
1. `suppress-author-with-locator` (9 styles)
2. `et-al-with-locator` (6 styles)
3. `with-locator` (3 styles)
4. `et-al-single-long-list` (3 styles)
5. `disambiguate-add-names-et-al` (3 styles)
6. `locator-section-with-suffix` (2 styles)

Interpretation:
- The `et-al`/disambiguation cluster was materially reduced by migrate-side
  processing extraction fixes.
- The next repeated citation gap is now dominated by
  `suppress-author-with-locator`, which should drive the next promotion.

## Locator Preservation Checkpoint

Second promotion applied:
- `crates/csln_migrate/src/main.rs`
  - inject `locator` into author-date citation templates when legacy citation
    layout uses locator but inferred templates omit it
- `scripts/merge-migration.js`
  - preserve locator component from base migration output during merge when
    inferred citation template has no locator

Wave rerun aggregate after this pass:
- Citations: `138/156` (88.5%) `(+9 from checkpoint 1, +24 from baseline)`
- Bibliography: `344/385` (89.4%) `(no regression)`
- Combined: `482/541` (89.1%) `(+9 from checkpoint 1, +24 from baseline)`

Citation mismatch clusters after locator preservation:
1. `et-al-with-locator` (4 styles)
2. `suppress-author-with-locator` (3 styles)
3. `et-al-single-long-list` (3 styles)
4. `disambiguate-add-names-et-al` (3 styles)
5. `with-locator` (2 styles)
6. `locator-section-with-suffix` (2 styles)

Interpretation:
- The `suppress-author-with-locator` cluster dropped from 9 styles to 3.
- Remaining misses are now primarily et-al/locator composition edge cases.

## Useful Commands
```bash
styles=(
  american-fisheries-society
  american-statistical-association
  american-marketing-association
  sage-harvard
  harvard-cite-them-right
  nlm-name-year
  new-harts-rules-author-date-space-publisher
  springer-basic-author-date-no-et-al
  springer-physics-author-date
  chicago-author-date-classic
  the-company-of-biologists
  modern-language-association
)

for s in "${styles[@]}"; do
  ./scripts/prep-migration.sh "styles-legacy/$s.csl" --agent
  node scripts/oracle.js "styles-legacy/$s.csl" --json > "/tmp/oracle-wave3-$s.json" || true
  jq -r '[.style, (.citations.passed // 0), (.citations.total // 0), (.bibliography.passed // 0), (.bibliography.total // 0)] | @tsv' \
    "/tmp/oracle-wave3-$s.json"
done
```
