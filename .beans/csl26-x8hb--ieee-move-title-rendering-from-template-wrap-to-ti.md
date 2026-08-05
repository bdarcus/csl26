---
# csl26-x8hb
title: 'ieee: move title rendering from template wrap to titles category config'
status: in-progress
type: task
priority: high
tags:
    - schema
    - style
    - fidelity
created_at: 2026-08-05T13:54:29Z
updated_at: 2026-08-05T13:54:36Z
parent: csl26-ccdt
---

`ieee.yaml` encoded a category-level policy (which works get quoted titles) in the template layer: `wrap: { punctuation: quotes }` on the base `title: primary`, applied to every type, then six near-identical type-variants existed purely to take the quotes back off.

Root cause: the style set `titles: humanities`, whose `component` rendering is plain, then compensated in the template. `TitlePreset::Ieee` already exists and quotes component titles.

## Why the template was the wrong layer

The engine keys title rendering by reference type, and the **substitute chain reads the same config** — `resolve_author_substitute` (values/contributor/substitute.rs:879) does not pass the contributor component's `Rendering` into the title branch, by design. A template `wrap:` could never reach a title substituting for a missing author; `options.titles` reaches both paths from one declaration.

## Change

- Replaced `titles: humanities` with explicit category config plus `type-mapping`, derived from the oracle's actual per-type treatment rather than guessed.
- Deleted `wrap: quotes` from the base template.
- Deleted four variants that only removed it (broadcast, dataset, report, personal_communication -- the last byte-identical to the base).
- Converted `book`, `motion-picture`, `entry-encyclopedia` from full templates to diffs; with the wrap gone they differ from base only by additions, which `modify` already expresses.

## Result

**361 -> 233 lines (-170/+42).** ieee exact parity **95/149 unchanged**, fidelity 1.0. Full embedded sweep 1558 -> 1559 (ASME +1 via inheritance; ieee is its direct parent). No regressions, no fidelity movement.

This is the evidence that withdrew the proposed `unset` and `abstract-variants` schema additions from PR #1142: once the rule sits in the right layer, nothing needs to clear an inherited field.

## Todo

- [x] Derive per-type title treatment from the oracle rather than guessing
- [x] Move rendering policy to options.titles with type-mapping
- [x] Delete redundant variants; convert the rest to diffs
- [x] just pre-commit
- [x] Full embedded sweep vs baseline
- [ ] CI green
