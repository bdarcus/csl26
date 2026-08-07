---
# csl26-7jht
title: Tune chicago-shortened-notes-bibliography to full fidelity
status: scrapped
type: task
priority: high
created_at: 2026-06-30T18:46:09Z
updated_at: 2026-08-07T13:21:38Z
parent: csl26-h7oc
blocked_by:
    - csl26-lxy3
---

Tune `chicago-shortened-notes-bibliography` to 100% fidelity + clean SQI via
the `style-tune` skill, against the shared Chicago corpus
(`chicago-18th-citations.json`, 15 items; `chicago-18th.json`, 402 refs).

## Baseline (measured 2026-06-30)
- citations: 6/15 (40%) — lowest of the four variants
- bibliography: 264/402 (66%)
- gated via `chicago-shared-corpus`, `min_pass_rate: 0.64` (csl26-h7oc)

## Input contract (style-tune)
- Embedded style ID: `chicago-shortened-notes-bibliography`
- Legacy CSL: `styles-legacy/chicago-fullnote-bibliography.csl` (verify exact
  source — shortened-note variant of the notes-bibliography family)
- Citum YAML: `crates/citum-schema-style/embedded/styles/chicago-shortened-notes-bibliography.yaml`
- Authority: CMOS 18 notes-bibliography system, shortened-note form
- Extends (via `-core`): `chicago-notes-18th` — re-baseline only after that
  bean lands, then tune the shortened-note + bibliography-specific deltas

## Why last
`chicago-shortened-notes-bibliography-core` extends `chicago-notes-18th`;
doing this after notes is tuned means the inherited citation baseline is
already improved, leaving this bean to focus on its own bibliography surface
(which notes-18th doesn't have) and shortened-note-specific deltas.

## Todo
- [ ] Re-run baseline once chicago-notes-18th tune lands (inherited citation
      numbers will have moved)
- [ ] Fidelity loop on the bibliography surface + shortened-note-specific
      citation deltas
- [ ] SQI loop
- [ ] style-qa-reviewer handoff (tier: embedded-core)

## 2026-07-30: oracle text-parity evidence (worst style in the portfolio: 2.4% parity, 0.751 fidelity)

Full-portfolio oracle-parity clustering (references-expanded run, report at fb0a01af) shows chicago-shortened-notes-bibliography's bibliography failure shape is systemic, not per-entry — comma delimiters where Chicago wants periods, quotes dropped, terms lowercased, across nearly every sampled entry:

```
O: Kafka, Franz. Metamorphosis. Translated by David Wyllie. Kurt Wolff Verlag, 1915.
C: Kafka, Franz, Metamorphosis. translated by David Wyllie, Kurt Wolff Verlag, 1915.

O: NASA Goddard Institute for Space Studies. \"Global Temperature Anomalies 1880-2023.\" NASA, 2024. https://data.giss.nasa.gov/gistemp/.
C: NASA Goddard Institute for Space Studies, Global Temperature Anomalies 1880-2023, NASA, 2024, https://data.giss.nasa.gov/gistemp/

O: State of JS Team. \"The State of JavaScript 2023.\" 2023. https://stateofjs.com/2023.
C: State of JS Team, The State of JavaScript 2023, 2023, https://stateofjs.com/2023
```

This style is `chicago-shortened-notes-bibliography-core.yaml` (284 lines) extending `chicago-notes-18th` (731 lines). Given `303a38f0 feat(schema)!: deep-merge options on extends` and `0b8efb77 fix(styles): fix two title deep-merge regressions` landed recently, this looks like deep-merge fallout from a shared root cause (delimiter/quoting/case options not surviving the extends chain), not N independent per-type bugs — worth checking the resolved merged template against `chicago-notes-18th`'s bibliography delimiters/affixes before doing per-entry tuning. Not independently root-caused — recorded as a strong lead for this bean's fidelity loop. Full clustering data path is stale (/tmp/prd_report.json); regenerate via `node scripts/report-core.js --styles chicago-shortened-notes-bibliography --parallelism 2`. See [[csl26-arly]].



## 2026-07-31 shared terminal-link punctuation progress

Enabled `bibliography.options.entry-suffix-after-url` and `entry-suffix-after-doi` in the shared shortened-notes bibliography core, so terminal URLs and DOIs receive its declared `entry-suffix: .`. The current broad exact-parity fixture remains **11/465**: no broad-match count lifted because its terminal-link entries also have structural template differences. This is shared punctuation progress only; it does not start or complete this task’s full tuning loop.



Direct verification: `just workflow-test styles-legacy/chicago-shortened-notes-bibliography.csl` passed **20/20 citations** and **46/46 bibliography entries** against citeproc-js after the suffix-policy change.

## Reasons for Scrapping

Superseded by the cluster-driven restructuring in
docs/specs/CHICAGO_FAMILY_STRATEGY.md (2026-08-07), for the same reason as
csl26-giun (scrapped alongside this bean): this style's worst-in-portfolio
parity (2.4% at last measurement) traces to systemic per-entry punctuation/
quoting/case drift, not per-entry defects, and per-style tuning against one
variant in isolation can't converge when the defect classes are shared across
the family (this style inherits chicago-notes-18th's citation baseline
directly).

Not deleted — preserved as evidence for the successor cluster beans (children
of csl26-h7oc): csl26-87yl (title quoting), csl26-vf5x (container-title
punctuation), csl26-yqma (name-list conjunction punctuation), and the shared
terminal-link punctuation work already landed (entry-suffix-after-url/-doi)
stays as a completed baseline these clusters build on.
