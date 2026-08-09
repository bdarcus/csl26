# Style Compatibility Inheritance Report Specification

**Status:** Active
**Version:** 1.3
**Date:** 2026-08-09
**Related:** beans `csl26-zik7`, `csl26-4hjr`, `csl26-t6dg`, `csl26-6th8`,
`csl26-hk3u`; [`STYLE_TAXONOMY.md`](./STYLE_TAXONOMY.md),
[`STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md`](./STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md)

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
- tri-state bibliography pairing with evidence-run attribution
- graceful reporting for missing parents, inheritance cycles, and absent
  measurement artifacts
- explicit coverage-audit registration, JSON exposure, and an audit-first
  explorer for registered styles

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
| `exactMatch` | `true` or `false` for paired text; `null` when text is not comparable |
| `exactAdjudication` | `matched`, `unresolved`, or `not-comparable`; parity alone never assigns fault |

Exact-text normalization removes transport-only carrier markup and collapses
whitespace. It preserves every visible text character, including citation and
bibliography numbering, case, punctuation, brackets, and role labels.
Consequently, differences such as `[1750?]` versus `1750?`, `Eds.` versus
`eds.`, a present versus absent role label, and differing numeric labels remain
visible.

The existing normalized `expected` and `actual` values, `match`, `similarity`,
and `caseMismatch` remain unchanged. Exact parity is calculated before
registered divergences and is informational only.

Oracle entries expose both raw and exact-normalized text plus `exactMatch`.
Style and portfolio summaries expose oracle-text matched, total, percentage,
unadjudicated status, not-comparable count, and non-gating state independently
of compatibility totals. Style records expose
`compatibilityScore`; the legacy `fidelityScore` field remains an equivalent
compatibility alias for consumers that already depend on it.

### Bibliography pairing contract

Bibliography observations have one of three comparison states:

| State | Pairing evidence | `match` | `exactMatch` | Metric treatment |
|---|---|---:|---:|---|
| Paired | Item ID or accepted similarity pairing | Boolean | Boolean | Included in compatibility and exact parity |
| Heuristic-unpaired | ID-less output left over after similarity pairing | `null` | `null` | Excluded from both metrics |
| ID-proven one-sided | An item ID exists on only one side | `false` | `null` | Compatibility failure; excluded from exact parity |

Each entry records its comparison state, pairing method (`id`, `similarity`, or
`position`), compatibility and exact-parity eligibility, and evidence-run ID,
label, and authority. A
heuristic leftover is evidence that pairing is unresolved, not proof that
either renderer omitted or added an item. ID-proven one-sided output is an
output-cardinality defect but provides no text pair to compare.

Style and portfolio summaries separately count paired observations,
heuristic-unpaired observations, ID-proven oracle-only observations, and
ID-proven Citum-only observations. Compatibility summaries also expose
unresolved-pairing counts. Exact-parity summaries expose `notComparable`.

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
verdict, compatibility score, and unadjudicated oracle text parity. Search and
sorting retain family context. Detail tables mark compatibility-matched rows
whose oracle text drifts, display the first differing span inline, and retain
the complete strings as hover text. This includes the Chicago author-date
guessed-year result.

Bibliography evidence is grouped by its actual run instead of being presented
as one synthetic sequence. Each run contains:

1. paired text findings with compatibility and exact-parity signals;
2. ID-proven cardinality failures with one compatibility-failure signal and
   exact parity shown as N/A; and
3. a collapsed neutral “Unpaired outputs—pairing unresolved” diagnostic for
   heuristic leftovers, with no failure icons or text-diff highlighting.

Render-only native smoke tests are labeled as having no oracle comparison and
never supply oracle-comparison rows. Bibliography findings identify their
configured authority and evidence-run label.

Coverage audits are opt-in report metadata, not directory discovery. A
registered style exposes a `coverageAudit` object in report JSON and replaces
its citation and bibliography diff tables with an audit-first explorer. The
explorer shows rendered, fallback, suppressed, uncovered, excluded, and joined
exact-parity summaries; filters output groups by surface, disposition,
comparison state, and review need; lists field dispositions with stable
observation IDs; and keeps exact Oracle/Citum differences collapsed beneath
each mismatching output. It links only to the human maintainer adjudication,
never to raw packet JSON. Supplemental benchmark summaries remain visible.

Uncovered observations are structural investigation leads. The explorer must
not present them as proof that a field caused a text mismatch. Styles without
a registered audit retain the existing diff-based evidence view unchanged.

## Implementation Notes

The embedded style registry is the canonical source for registry kind and
aliases. Existing CSL Reach remains the dependent-style impact measure.
Measurement artifacts are supporting evidence and do not change report gates.

The Chicago source item `issued: 1750?` converts faithfully to native EDTF
`1750?`. Rendering paired uncertainty markers requires a separate schema and
engine change; this report only exposes and tracks that mismatch.

## Acceptance Criteria

- [x] `compareText` and oracle entries expose raw and exact-parity data while
  existing compatibility behavior remains unchanged.
- [x] Tests cover bracket, punctuation, case, role-label, and missing-label
  drift plus unchanged strict grading and registered-divergence behavior.
- [x] Report discovery records complete chains, wrapper forms, registry aliases,
  optional evidence, missing parents, and cycles.
- [x] Family aggregates and ordering are deterministic and based on aggregate
  CSL reach.
- [x] The generated dashboard groups families and displays compatibility and
  unadjudicated oracle text parity independently, with the first differing
  span visible inline.
- [x] The live Chicago bibliography row is compatibility-matched and
  exact-parity-failed for `[1750?]` versus `1750?`.
- [x] Heuristic-unpaired observations are neutral and excluded from both
  metrics; ID-proven one-sided observations fail compatibility only.
- [x] Bibliography evidence is grouped by run, and native smoke evidence stays
  render-only.
- [x] Full report generation, baseline validation, browser QA, and repository
  hygiene checks pass.
- [x] The report registers Chicago explicitly, exposes its `coverageAudit`
  object, renders the accessible responsive explorer, and preserves the
  existing view for unregistered styles.

## Changelog

- v1.3 (2026-08-09): Added explicit coverage-audit registration, report JSON,
  the audit-first explorer, human adjudication linking, and unchanged fallback
  behavior for unregistered styles.
- v1.2 (2026-07-28): Added tri-state bibliography pairing, nullable comparison
  fields, evidence-run grouping, and neutral unpaired diagnostics.
- v1.1 (2026-07-28): Preserved visible numbering, added unresolved
  adjudication state, and required first-difference rendering.
- v1.0 (2026-07-28): Initial specification, activated with the report
  implementation and validation.
