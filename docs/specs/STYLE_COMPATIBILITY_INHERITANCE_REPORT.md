# Style Compatibility Inheritance Report Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-07-28
**Related:** beans `csl26-zik7`, `csl26-4hjr`; [`STYLE_TAXONOMY.md`](./STYLE_TAXONOMY.md)

## Purpose

The compatibility report must show both behavioral compatibility and the
structure of the style portfolio. It must make inheritance, aliases, measured
family relationships, and exact textual drift visible without weakening or
changing the existing compatibility gates.

## Scope

In scope:

- discovery of complete `extends` chains, including hidden core and base styles
- implementation-form classification for public styles
- registry identity, alias, and family-reach reporting
- optional behavioral-band and delta-derivability evidence
- family-grouped JSON and HTML report output
- an informational exact-text parity tier alongside lenient compatibility
- graceful reporting for missing parents, inheritance cycles, and absent
  measurement artifacts

Out of scope:

- changing the existing lenient match, similarity, case-mismatch, adjusted
  divergence, or CI gating semantics
- changing the style schema, date renderer, Chicago YAML, or legacy CSL input
- treating exact parity as byte-identical HTML comparison
- inferring behavioral equivalence when no measurement artifact is available

## Design

### Comparison contract

`compareText` keeps the meanings of its existing fields and adds:

| Field | Meaning |
|---|---|
| `rawExpected` | Unmodified benchmark renderer output |
| `rawActual` | Unmodified Citum renderer output |
| `exactExpected` | Benchmark output after exact-text normalization |
| `exactActual` | Citum output after exact-text normalization |
| `exactMatch` | Whether the exact-normalized values are identical |

Exact-text normalization removes carrier markup and bibliography numbering and
collapses whitespace. It preserves case, punctuation, brackets, and role
labels. Consequently, differences such as `[1750?]` versus `1750?`, `Eds.`
versus `eds.`, and a present versus absent role label remain visible.

The existing normalized `expected` and `actual` values, `match`, `similarity`,
and `caseMismatch` remain unchanged. Exact parity is calculated before
registered divergences and is informational only.

Oracle entries expose both raw and exact-normalized text plus `exactMatch`.
Style and portfolio summaries expose exact matched, total, and percentage
counts independently of compatibility totals.

### Inheritance contract

Each public report style records:

- its direct parent, complete inheritance chain, and ultimate family root
- whether the chain is complete, missing a parent, or cyclic
- implementation form: `standalone`, `config-wrapper`, or
  `structural-wrapper`
- registry kind, aliases, and alias count
- optional behavioral near-clone band and derivability evidence
- compatibility and exact-parity summaries

A style without `extends` is `standalone`. A style with `extends` and no local
rendering structure is a `config-wrapper`. A style with `extends` and local
templates or type variants is a `structural-wrapper`.

Hidden core and base styles participate in chain and family-root discovery but
do not become standalone compatibility rows.

### Family contract

The top-level report contains family aggregates. A family is keyed by the
ultimate reachable root. Its aggregate CSL reach is the sum of the existing
dependent-style reach values for its public members. Families are ordered by:

1. aggregate CSL reach, descending
2. family-root name, ascending

Members are ordered deterministically by style name. Family summaries include
member and alias counts, compatibility, and exact parity.

### Optional evidence

The report loads the latest matching behavioral-band and delta-derivability
artifacts from `scripts/report-data/`. Evidence is joined by candidate style
identifier. Missing artifacts or missing rows yield an explicit unavailable
state rather than a report failure.

### Dashboard behavior

`docs/compat.html` groups public rows under family headings and exposes aliases,
the inheritance chain, implementation form, near-clone band, derivability
verdict, compatibility score, and exact parity. Search and sorting retain
family context. Detail panels list compatibility-matched rows whose exact text
drifts, including the Chicago author-date guessed-year result.

## Implementation Notes

The embedded style registry is the canonical source for registry kind and
aliases. Existing CSL Reach remains the dependent-style impact measure.
Measurement artifacts are supporting evidence and do not change report gates.

The Chicago source item `issued: 1750?` converts faithfully to native EDTF
`1750?`. Rendering paired uncertainty markers requires a separate schema and
engine change; this report only exposes and tracks that mismatch.

## Acceptance Criteria

- [ ] `compareText` and oracle entries expose raw and exact-parity data while
  existing compatibility behavior remains unchanged.
- [ ] Tests cover bracket, punctuation, case, role-label, and missing-label
  drift plus unchanged strict grading and registered-divergence behavior.
- [ ] Report discovery records complete chains, wrapper forms, registry aliases,
  optional evidence, missing parents, and cycles.
- [ ] Family aggregates and ordering are deterministic and based on aggregate
  CSL reach.
- [ ] The generated dashboard groups families and displays compatibility and
  exact parity independently.
- [ ] The live Chicago bibliography row is compatibility-matched and
  exact-parity-failed for `[1750?]` versus `1750?`.
- [ ] Full report generation, baseline validation, browser QA, and repository
  hygiene checks pass.

## Changelog

- v1.0 (2026-07-28): Initial specification.
