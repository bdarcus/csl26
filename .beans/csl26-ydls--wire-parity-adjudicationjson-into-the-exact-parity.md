---
# csl26-ydls
title: Wire parity-adjudication.json into the exact-parity gate
status: todo
type: task
priority: normal
tags:
    - scripts
    - fidelity
    - reporting
created_at: 2026-08-23T21:29:07Z
updated_at: 2026-08-23T21:29:07Z
parent: csl26-w0hf
---

Discovered while seeding scripts/report-data/parity-adjudication.json during the 2026-08-23 Chicago parity leverage audit (docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md): the ledger has states (citeproc-correct/unclear/citum-correct) and check-core-quality.js validates entry shape and reports counts, but nothing subtracts adjudicated rows from report-core.js's exactParity.total. Writing an entry today records a decision with no effect on any reported number.

## Why this isn't a small filter

- The existing analogous mechanism, known-divergences.json, isn't a simple ID lookup -- it's bespoke per-divergence detector functions in scripts/lib/oracle-divergences.js (detectDiv004OrderDifference, explainCitationMismatchFromDiv010, etc.), each pattern-matching entry content. A (style, id) lookup against parity-adjudication.json would be simpler than that in principle, but it is still new logic inside summarizeExactParity() (scripts/report-core.js), the function every embedded style's hard CI gate reads.
- check-core-quality.js's parityTotalDrift check (lines ~396-403) hard-fails CI (process.exit(1)) the instant any style's exactParity.total differs from scripts/report-data/embedded-parity-baseline.json. Wiring the ledger in immediately breaks CI for every style with an adjudication entry unless the baseline is regenerated in the same change.
- scripts/derive-parity-baseline.js builds the *entire* baseline file from whichever styles are present in its input report -- it does not merge. Regenerating it correctly requires a full-portfolio node scripts/report-core.js --all-features run across the whole embedded tier, not a --style-scoped one, or every other embedded style silently drops out of the file.
- That full-portfolio sweep is the exact heavy operation flagged as unsafe to run unattended on the 14GB/8-core dev laptop (has crashed twice under default-concurrency corpus sweeps) -- it needs the sequential, memory-capped systemd-run approach, or a CI/beefier-machine run, not an ad hoc pass.

## Seed data already recorded

3 chicago styles, 7 entries total, all state 'unclear' (genre-slug fixture artifact -- references-expanded.json encodes some genre values as kebab-case slugs that citeproc-js echoes literally and Citum humanizes; not a processor defect). These entries are inert until this bean lands.

## Scope options (from the audit's follow-up discussion)

1. General mechanism: add the (style, id) lookup to summarizeExactParity, exclude unclear/citum-correct from total, count citeproc-correct as a required fail as documented. Then run the full-portfolio sweep (memory-capped, sequential) and regenerate embedded-parity-baseline.json for all embedded styles in the same commit.
2. Narrow one-off: hard-code the 7 already-known ids for the 3 already-touched styles only, regenerate only those 3 styles' baseline entries. Smaller blast radius, but leaves citeproc-correct/citum-correct unwired everywhere else and doesn't generalize -- would need redoing properly the next time any style gets an adjudication entry.

Option 1 is very likely the right target; recorded here rather than decided unilaterally since it changes what "exact parity" means for the CI gate portfolio-wide.
