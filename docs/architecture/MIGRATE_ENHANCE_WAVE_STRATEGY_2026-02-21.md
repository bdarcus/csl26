# Migrate+Enhance Wave Strategy (2026-02-21)

> Canonical status/next-actions now live in:
> `docs/architecture/MIGRATE_ENHANCE_WAVE_RUNBOOK_2026-02-21.md`

## Goal
Raise `csln-migrate` conversion quality toward consistently high fidelity and high SQI by selecting legacy styles that maximize migration learning signal, not just dependent-count coverage.

## Current Baseline Context
- Core catalog now includes ~100 styles in `styles/`.
- Top parent coverage and strict-match fidelity are already strong for major author-date and numeric families.
- Remaining unconverted parent backlog (excluding `apa`, already represented by `apa-7th`) is low-dependent but high-variance:
  - author-date: 70 styles, 175 dependents
  - numeric: 72 styles, 145 dependents
  - note: 56 styles, 86 dependents
  - author: 1 style, 5 dependents

## Selection Strategy
Use a weighted selection score for each remaining parent style:

1. Migration-learning value (highest weight)
- New structural patterns or condition logic likely to expose `csln-migrate` gaps.
- Examples: note position logic, ibid/subsequent behavior, no-url variants, no-et-al variants, label-page variants, alphabetical numeric ordering.

2. Family leverage
- Prefer clusters of related variants so one migration fix can improve many styles.
- Target clusters: Chicago Notes variants, New Hart's Rules note variants, AMA/NLM no-et-al/no-url variants.

3. Format balance
- Maintain a mixed wave: note + author-date + numeric (+ one author/label exemplar).
- This avoids overfitting migration heuristics to one format class.

4. Dependent coverage (secondary)
- Use dependents as tie-breaker between equally informative candidates.

## Proposed Conversion Waves

### Wave 1 (Note-heavy foundation, 12 styles)
Primary objective: teach migration pipeline note-style structure and variant toggles.

- chicago-notes-classic
- chicago-notes-publisher-place-archive-place-first-no-url
- chicago-notes-bibliography-17th-edition
- chicago-notes-classic-archive-place-first
- chicago-notes-classic-no-url
- chicago-notes-bibliography-classic-archive-place-first-no-url
- chicago-shortened-notes-bibliography-classic-archive-place-first
- new-harts-rules-notes
- new-harts-rules-notes-label-page
- new-harts-rules-notes-label-page-no-url
- mhra-notes-publisher-place
- mhra-notes-publisher-place-no-url

Expected migration learnings:
- bibliography/no-bibliography note behavior
- label-page rendering and suppression
- archive-place / publisher-place conditional output
- no-url toggle handling without template explosion

### Wave 2 (Numeric variant stress, 12 styles)
Primary objective: compress numeric variant logic into reusable migrate rules/presets.

- microbiology-society
- springer-humanities-brackets
- oxford-journals-scimed-numeric
- endocrine-press
- american-medical-association-no-et-al
- american-medical-association-no-url
- american-medical-association-alphabetical
- springer-basic-brackets-no-et-al
- springer-basic-brackets-no-et-al-alphabetical
- nlm-citation-sequence-superscript-year-only-no-issue
- nlm-citation-sequence-brackets-no-et-al
- nlm-citation-sequence-brackets-year-only-no-issue

Expected migration learnings:
- no-et-al, no-url, and year-only switches as options-level transforms
- alphabetical numeric sorting variants
- superscript/bracket behavior normalization

### Wave 3 (Author-date + author/label diversity, 12 styles)
Primary objective: improve migration robustness on humanities/social-science variants.

- american-fisheries-society
- american-statistical-association
- american-marketing-association
- sage-harvard
- harvard-cite-them-right
- nlm-name-year
- new-harts-rules-author-date-space-publisher
- springer-basic-author-date-no-et-al
- springer-physics-author-date
- chicago-author-date-classic
- the-company-of-biologists
- modern-language-association

Expected migration learnings:
- author-date punctuation and spacing variants
- name-year grouping and disambiguation edge behavior
- non-numeric label/author style handling (`modern-language-association`)

### Wave 4 (Deferred legal-note complexity)
Scope candidate: `oscola` and related legal/note outliers.

Reason to defer:
- likely requires processor/core legal-citation feature work beyond pure migrate template tuning
- higher risk of mixing migration and schema concerns in same iteration

## Execution Workflow (per wave)
Use styleauthor "Priority Batch Migrate+Enhance" workflow with a migrate-first variant.

1. Batch prep
- For each selected legacy style: `./scripts/prep-migration.sh styles-legacy/<style>.csl`
- Capture baseline oracle JSON per style.

2. Enhance loop (fidelity first, SQI second)
- Apply style edits to close oracle mismatches.
- Apply SQI-oriented compaction/preset extraction only after fidelity stabilizes.

3. Rerun comparison (required)
- Re-run migrate pipeline for same style set.
- Compare baseline vs edited vs rerun:
  - citation pass
  - bibliography pass
  - fidelity
  - SQI

4. Pattern extraction to `csln-migrate`
- Promote repeated fixes (2+ styles) into migration rules/presets.
- Re-run wave to confirm no fidelity regression.

5. Core regression gate
- `node scripts/report-core.js > /tmp/core-report.json`
- `node scripts/check-core-quality.js --report /tmp/core-report.json --baseline scripts/report-data/core-quality-baseline.json`

## Metrics and Stop Conditions
Per-style hard gate:
- Fidelity must not regress.

Per-wave targets:
- Edited wave fidelity >= 98% (goal: 100%)
- Auto-rerun fidelity improves against baseline by >= 5 points for that wave
- Median SQI for wave styles >= 85 with no fidelity regressions

Escalation rule:
- If bibliography match is <50% after first enhancement pass, escalate to template redesign before further tuning.

## Status Checkpoint (2026-02-21)
- Wave 1: completed and merged into branch iteration stream (`642/664`, 96.7% combined).
- Wave 2 baseline: `450/528` (85.2% combined).
- Wave 2 post-script checkpoint: `514/528` (97.3% combined).
- Wave 2 post-Rust `csln-migrate` checkpoint: `518/528` (98.1% combined).
- Wave 2 citations are now fully clean (`144/144`).

## Branch and PR Plan
- Branch: `codex/migrate-enhance-wave-strategy`
- Execution branches (planned):
  - `codex/migrate-enhance-wave1-note`
  - `codex/migrate-enhance-wave2-numeric`
  - `codex/migrate-enhance-wave3-author-date`
- PR approach:
  - one PR per wave for reviewable diffs
  - final synthesis PR for `csln-migrate` rule/preset extraction

## Assumptions (unless changed)
- Treat `apa` as already represented by `apa-7th`; do not spend a slot on `apa` alias conversion.
- Prioritize migration-learning coverage over raw dependent count from here.
- Defer legal-note (`oscola`) to a dedicated wave after note baseline improvements land.
