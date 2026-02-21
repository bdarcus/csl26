# Wave 1 Handoff: Note-Style Migrate+Enhance (2026-02-21)

> Canonical status/next-actions now live in:
> `docs/architecture/MIGRATE_ENHANCE_WAVE_RUNBOOK_2026-02-21.md`

## Branch
`codex/migrate-enhance-wave-strategy`

## Scope
Wave 1 note-heavy batch from
`docs/architecture/MIGRATE_ENHANCE_WAVE_STRATEGY_2026-02-21.md`.

## What Was Completed
1. Generated baseline CSLN styles for 12 note-focused legacy parents using:
`./scripts/prep-migration.sh styles-legacy/<style>.csl --agent`
2. Captured oracle baseline for the wave with note citations fixture:
`node scripts/oracle.js styles-legacy/<style>.csl --json --citations-fixture tests/fixtures/citations-note-expanded.json`
3. Applied a shared citation enhancement pattern across the affected styles:
   - patent number + issued date components
   - interview date + medium + interviewer components
4. Improved migration workflow efficiency by quieting infer-template log noise in
`/Users/brucedarcus/Code/csl26/scripts/prep-migration.sh` (logs now captured and surfaced only on failures).

## Generated Styles (Wave 1)
- `styles/chicago-notes-classic.yaml`
- `styles/chicago-notes-publisher-place-archive-place-first-no-url.yaml`
- `styles/chicago-notes-bibliography-17th-edition.yaml`
- `styles/chicago-notes-classic-archive-place-first.yaml`
- `styles/chicago-notes-classic-no-url.yaml`
- `styles/chicago-notes-bibliography-classic-archive-place-first-no-url.yaml`
- `styles/chicago-shortened-notes-bibliography-classic-archive-place-first.yaml`
- `styles/new-harts-rules-notes.yaml`
- `styles/new-harts-rules-notes-label-page.yaml`
- `styles/new-harts-rules-notes-label-page-no-url.yaml`
- `styles/mhra-notes-publisher-place.yaml`
- `styles/mhra-notes-publisher-place-no-url.yaml`

## Baseline Oracle Results

| Style | Citations | Bibliography |
|---|---:|---:|
| chicago-notes-classic | 33/34 | 0/0 |
| chicago-notes-publisher-place-archive-place-first-no-url | 32/34 | 0/0 |
| chicago-notes-bibliography-17th-edition | 33/34 | 30/32 |
| chicago-notes-classic-archive-place-first | 33/34 | 0/0 |
| chicago-notes-classic-no-url | 32/34 | 0/0 |
| chicago-notes-bibliography-classic-archive-place-first-no-url | 32/34 | 29/32 |
| chicago-shortened-notes-bibliography-classic-archive-place-first | 33/34 | 30/32 |
| new-harts-rules-notes | 31/34 | 29/32 |
| new-harts-rules-notes-label-page | 31/34 | 29/32 |
| new-harts-rules-notes-label-page-no-url | 31/34 | 29/32 |
| mhra-notes-publisher-place | 32/34 | 29/32 |
| mhra-notes-publisher-place-no-url | 32/34 | 29/32 |

Wave aggregate:
- Citations: `385/408` (94.4%)
- Bibliography: `234/256` (91.4%)
- Combined fidelity: `619/664` (93.2%)

## Enhancement Pass Results

| Style | Citations | Bibliography |
|---|---:|---:|
| chicago-notes-classic | 34/34 | 0/0 |
| chicago-notes-publisher-place-archive-place-first-no-url | 34/34 | 0/0 |
| chicago-notes-bibliography-17th-edition | 34/34 | 30/32 |
| chicago-notes-classic-archive-place-first | 34/34 | 0/0 |
| chicago-notes-classic-no-url | 34/34 | 0/0 |
| chicago-notes-bibliography-classic-archive-place-first-no-url | 34/34 | 29/32 |
| chicago-shortened-notes-bibliography-classic-archive-place-first | 34/34 | 30/32 |
| new-harts-rules-notes | 34/34 | 29/32 |
| new-harts-rules-notes-label-page | 34/34 | 29/32 |
| new-harts-rules-notes-label-page-no-url | 34/34 | 29/32 |
| mhra-notes-publisher-place | 34/34 | 29/32 |
| mhra-notes-publisher-place-no-url | 34/34 | 29/32 |

Wave aggregate after enhancement:
- Citations: `408/408` (100.0%)  `(+23)`
- Bibliography: `234/256` (91.4%) `(no regression)`
- Combined fidelity: `642/664` (96.7%) `(+23)`

## Remaining Mismatch Cluster
Citation mismatches are fully resolved for this wave.

Remaining gap is bibliography fidelity (`234/256`), concentrated in
style-specific note bibliography formatting differences.

## Resume Checklist
1. Re-run `prep-migration` for the same 12 styles and capture rerun metrics.
2. Compare baseline vs edited vs rerun for citation + bibliography + SQI.
3. Extract reusable migrate heuristics from the wave:
   - no-author title-only note citations
   - conference author-shortening thresholds in note mode
   - note-style patent/interview citation tails
4. Start Wave 2 numeric variant stress batch.

## Useful Commands
```bash
# Regenerate wave baseline table
styles=(
  chicago-notes-classic
  chicago-notes-publisher-place-archive-place-first-no-url
  chicago-notes-bibliography-17th-edition
  chicago-notes-classic-archive-place-first
  chicago-notes-classic-no-url
  chicago-notes-bibliography-classic-archive-place-first-no-url
  chicago-shortened-notes-bibliography-classic-archive-place-first
  new-harts-rules-notes
  new-harts-rules-notes-label-page
  new-harts-rules-notes-label-page-no-url
  mhra-notes-publisher-place
  mhra-notes-publisher-place-no-url
)
for s in "${styles[@]}"; do
  node scripts/oracle.js "styles-legacy/$s.csl" --json \
    --citations-fixture tests/fixtures/citations-note-expanded.json \
    > "/tmp/oracle-$s.json" || true
  jq -r '[.style, (.citations.passed // 0), (.citations.total // 0), (.bibliography.passed // 0), (.bibliography.total // 0)] | @tsv' \
    "/tmp/oracle-$s.json"
done
```
