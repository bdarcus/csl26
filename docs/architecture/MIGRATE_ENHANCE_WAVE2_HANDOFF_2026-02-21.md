# Wave 2 Handoff: Numeric Variant Stress (2026-02-21)

## Branch
`codex/migrate-enhance-wave-strategy`

## Scope
Wave 2 from
`docs/architecture/MIGRATE_ENHANCE_WAVE_STRATEGY_2026-02-21.md`.

Goal: improve `csln-migrate` behavior on numeric variant families
(`no-et-al`, `no-url`, `alphabetical`, `year-only`, bracket/superscript variants).

## Batch Styles (12)
- `styles/microbiology-society.yaml`
- `styles/springer-humanities-brackets.yaml`
- `styles/oxford-journals-scimed-numeric.yaml`
- `styles/endocrine-press.yaml`
- `styles/american-medical-association-no-et-al.yaml`
- `styles/american-medical-association-no-url.yaml`
- `styles/american-medical-association-alphabetical.yaml`
- `styles/springer-basic-brackets-no-et-al.yaml`
- `styles/springer-basic-brackets-no-et-al-alphabetical.yaml`
- `styles/nlm-citation-sequence-superscript-year-only-no-issue.yaml`
- `styles/nlm-citation-sequence-brackets-no-et-al.yaml`
- `styles/nlm-citation-sequence-brackets-year-only-no-issue.yaml`

## What Was Completed
1. Generated all 12 styles via:
`./scripts/prep-migration.sh styles-legacy/<style>.csl --agent`
2. Captured baseline oracle metrics per style via:
`node scripts/oracle.js styles-legacy/<style>.csl --json`

## Baseline Oracle Results

| Style | Citations | Bibliography |
|---|---:|---:|
| microbiology-society | 7/12 | 31/32 |
| springer-humanities-brackets | 7/12 | 30/32 |
| oxford-journals-scimed-numeric | 7/12 | 31/32 |
| endocrine-press | 7/12 | 31/32 |
| american-medical-association-no-et-al | 5/12 | 31/32 |
| american-medical-association-no-url | 5/12 | 31/32 |
| american-medical-association-alphabetical | 3/12 | 31/32 |
| springer-basic-brackets-no-et-al | 7/12 | 31/32 |
| springer-basic-brackets-no-et-al-alphabetical | 7/12 | 31/32 |
| nlm-citation-sequence-superscript-year-only-no-issue | 7/12 | 32/32 |
| nlm-citation-sequence-brackets-no-et-al | 7/12 | 32/32 |
| nlm-citation-sequence-brackets-year-only-no-issue | 7/12 | 32/32 |

Wave aggregate baseline:
- Citations: `76/144` (52.8%)
- Bibliography: `374/384` (97.4%)
- Combined: `450/528` (85.2%)

## Post-Enhancement Checkpoint (script-level)

After enhancing `scripts/merge-migration.js` and re-running the same 12-style
batch, results improved to:

- Citations: `140/144` (97.2%)
- Bibliography: `374/384` (97.4%)
- Combined: `514/528` (97.3%)

Per-style citation results after enhancement:

| Style | Citations | Bibliography |
|---|---:|---:|
| microbiology-society | 12/12 | 31/32 |
| springer-humanities-brackets | 12/12 | 30/32 |
| oxford-journals-scimed-numeric | 12/12 | 31/32 |
| endocrine-press | 12/12 | 31/32 |
| american-medical-association-no-et-al | 12/12 | 31/32 |
| american-medical-association-no-url | 12/12 | 31/32 |
| american-medical-association-alphabetical | 9/12 | 31/32 |
| springer-basic-brackets-no-et-al | 12/12 | 31/32 |
| springer-basic-brackets-no-et-al-alphabetical | 11/12 | 31/32 |
| nlm-citation-sequence-superscript-year-only-no-issue | 12/12 | 32/32 |
| nlm-citation-sequence-brackets-no-et-al | 12/12 | 32/32 |
| nlm-citation-sequence-brackets-year-only-no-issue | 12/12 | 32/32 |

## Post-Rust Checkpoint (`csln-migrate` enhancement)

After adding Rust-side bibliography sort extraction in `csln-migrate`,
refining group-sort author behavior in `csln-processor`, and rerunning Wave 2:

- Citations: `144/144` (100.0%)
- Bibliography: `374/384` (97.4%)
- Combined: `518/528` (98.1%)

Per-style citation deltas from this Rust pass:
- `american-medical-association-alphabetical`: `9/12` -> `12/12`
- `springer-basic-brackets-no-et-al-alphabetical`: `11/12` -> `12/12`

Remaining citation misses:
- none in Wave 2 (`12/12` citations for all 12 styles)

Rust changes implemented:
- Extract legacy bibliography sort into CSLN `bibliography.sort` (`GroupSort`).
- Preserve legacy numeric bibliography sort extraction broadly.
- In group sorting, apply author->title fallback only when the sort template
  includes a title key; otherwise, missing-name entries sort last.

## Citation Mismatch Clusters (Baseline)
Repeated across the full 12-style batch:
1. `single-with-prefix-and-suffix` (12 styles)
2. `multi-item-with-locators` (12 styles)
3. `multi-item` (12 styles)
4. `mixed-visibility-and-prefix` (12 styles)
5. `locator-section-with-suffix` (12 styles)

Secondary cluster:
- `with-locator` (3 styles)
- `et-al-with-locator` (3 styles)

Interpretation:
- Wave 2 citation failures are primarily cluster/locator/prefix-suffix composition errors,
  not bibliography ordering errors.
- This is a strong signal that migration citation template selection for numeric variants
  needs reusable fixes in shared citation logic.

## Recommended Fix Order
1. Start with one representative style from each subfamily:
   - AMA variant: `styles/american-medical-association-no-et-al.yaml`
   - Springer brackets variant: `styles/springer-basic-brackets-no-et-al.yaml`
   - NLM year-only variant: `styles/nlm-citation-sequence-brackets-year-only-no-issue.yaml`
2. Fix cluster/locator rendering parity first (all 5 common failing IDs).
3. Propagate to remaining 9 styles.
4. Re-run full 12-style oracle.
5. Capture edited vs rerun comparison and extract migrate heuristics.

## Useful Commands
```bash
styles=(
  microbiology-society
  springer-humanities-brackets
  oxford-journals-scimed-numeric
  endocrine-press
  american-medical-association-no-et-al
  american-medical-association-no-url
  american-medical-association-alphabetical
  springer-basic-brackets-no-et-al
  springer-basic-brackets-no-et-al-alphabetical
  nlm-citation-sequence-superscript-year-only-no-issue
  nlm-citation-sequence-brackets-no-et-al
  nlm-citation-sequence-brackets-year-only-no-issue
)

for s in "${styles[@]}"; do
  node scripts/oracle.js "styles-legacy/$s.csl" --json > "/tmp/oracle-wave2-$s.json" || true
  jq -r '[.style, (.citations.passed // 0), (.citations.total // 0), (.bibliography.passed // 0), (.bibliography.total // 0)] | @tsv' \
    "/tmp/oracle-wave2-$s.json"
done
```
