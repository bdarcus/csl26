---
# csl26-lxy3
title: Tune chicago-notes-18th to full fidelity
status: todo
type: task
priority: high
created_at: 2026-06-30T18:46:08Z
updated_at: 2026-08-02T14:19:30Z
parent: csl26-h7oc
blocked_by:
    - csl26-ucg3
    - csl26-shco
    - csl26-2nv1
---

Tune `chicago-notes-18th` to 100% fidelity + clean SQI via the `style-tune`
skill, against the shared Chicago corpus citation surface
(`chicago-18th-citations.json`, 15 items — this style has no bibliography
surface; `bibliography.template: []` is intentional).

## Baseline (measured 2026-06-30)
- citations: 7/15 (47%) — lowest of the four variants
- gated via `chicago-shared-corpus`, `min_pass_rate: 0.46` (csl26-h7oc)
- bibliography: n/a (notes-only style by design)

## Input contract (style-tune)
- Embedded style ID: `chicago-notes-18th`
- Legacy CSL: `styles-legacy/chicago-notes.csl`
- Citum YAML: `crates/citum-schema-style/embedded/styles/chicago-notes-18th.yaml`
- Authority: CMOS 18 notes system
- Extends: `chicago-18-base` (csl26-zs0f)

## Related
`csl26-ucg3` (chicago notes legal/treaty note-flow, Bluebook) covers a known
defect cluster in this surface (stray leading comma on author-less legal
cases; repeated treaty fields; double year/pages in conference notes). Fold
its fixes into this tune pass rather than tracking separately — needs a
content/format decision with the maintainer per its bean before changing
Bluebook-specialised output.

## Why third (parallel start with author-date, blocks shortened)
No inheritance dependency on author-date/T&F, so this can start immediately.
`chicago-shortened-notes-bibliography-core` extends this style, so notes
should land before shortened starts, to avoid shortened inheriting an
unfinished citation surface.

## Todo
- [ ] Resolve csl26-ucg3's content/format questions with the maintainer first
      (Bluebook legal-case/treaty/conference note-flow)
- [ ] Fidelity loop on the citation surface, until 100% on the shared corpus
- [ ] SQI loop
- [ ] style-qa-reviewer handoff (tier: embedded-core)
