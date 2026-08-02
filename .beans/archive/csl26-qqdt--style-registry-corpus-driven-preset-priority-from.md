---
# csl26-qqdt
title: 'schema-style: corpus-driven preset discovery for config concerns'
status: completed
type: task
priority: normal
tags:
    - registry
    - presets
    - citum-analyze
created_at: 2026-06-16T15:49:15Z
updated_at: 2026-08-02T18:16:47Z
---

The `--config-presets` mode (csl26-t56t) discovers per-concern config
shapes (contributors, dates, titles, locators) across the CSL corpus that do
not match any existing named preset in citum-schema-style.

This bean is an audit task, not a count-only implementation task. The analyzer
is reliable as an exact serialized-shape frequency report, but it is not an
automatic missing-preset detector. Review the report and classify recurring
candidates as `accept`, `defer`, or `reject`.

Current evidence from the tree:

- `2844` styles analyzed, `0` parse errors.
- `dates`: `770` matched, `0` unmatched. Do not add date presets; current
  date presets cover all extracted non-default date configs.
- `locators`: `120` unmatched, one recurring shape: the current author-date
  locator config plus `strip-label-periods: true`. Treat this as the only
  obvious implementation candidate, pending final preset naming.
- `titles`: `643` matched, `1697` unmatched across `29` recurring shapes.
  Treat these as taxonomy/design evidence first, especially `default.emph`,
  `default.text-case: title`, and mixed default/category overrides.
- `contributors`: `1931` matched, `909` unmatched across `64` recurring
  shapes. Treat these as style-family or convention evidence; require a
  recognizable family/convention cluster before adding new `ContributorPreset`
  variants.

Priority order: rank by `corpus_count`, style-family/convention coherence,
naming clarity, and expected authored YAML reduction. Do not promote a shape
only because it is frequent.

Suggested next step: build a short audit table for the top contributor and title
candidates with count, examples, likely family/convention, and classification
(`accept`, `defer`, or `reject`). Do not implement contributor or title presets
from this report until that audit identifies a nameable family or convention.

Run:

```bash
cargo run --bin citum-analyze -- styles-legacy --config-presets --json \
  | jq '.concerns[] | {concern, matched_style_count, unmatched_style_count, candidate_count: (.candidates | length), candidates: .candidates[:5]}'
```

Public API impact for this bean revision: none. If implementation is approved
later, expected API surface is limited to a possible new `LocatorPreset` variant
for the `strip-label-periods: true` shape. Add no new `DatePreset`, `TitlePreset`,
or `ContributorPreset` without a separate taxonomy decision.

For a later implementation bean, add schema parse/resolve tests for any new
preset, add analyzer reverse-match coverage so the accepted candidate no longer
appears unmatched, and run `just pre-commit` for Rust/schema changes.

## Audit Outcome (csl26-4aml, 2026-08-02)

Ran the analyzer, found and fixed two structural defects before drawing conclusions:

1. `ContributorConfig.delimiter` serializes `None` as `"delimiter": null` while presets omit the
   field entirely for the default value (`skip_serializing_if`) — a normalization artifact
   fragmenting the contributor candidate list. Fixed in the analyzer (strip null-valued keys before
   hashing, both extracted and preset sides).
2. No `TitlePreset` ever sets `TitlesConfig.default` (only component/monograph/periodical/serial),
   so any `default`-only extracted shape was structurally unreachable — not a taxonomy gap.

Re-ran after the fix:
- **locators**: 120/120 now matched (added `LocatorPreset::Numeric` = author-date + strip-label-periods).
- **titles**: matched rose 643→1681, unmatched fell 1697→659 (added `TitlePreset::EmphasisAll` `default.emph`, `TitlePreset::TitleCase` `default.text-case:title`).
- **contributors**: matched_style_count unchanged (1931, identical before/after the null-normalization fix) — the delimiter:null artifact, while real, was never the sole blocker for any candidate shape; every remaining candidate differs from its nearest preset by ≥2 other fields, scattered across many different presets. **No coherent family/convention cluster found.** Per this bean's original guidance, no `ContributorPreset` variants added.

Also added a nearest-preset diff and `share_of_unmatched` to the JSON output, and `pub const ALL` on each preset enum (was a hand-maintained, untested mirror list — silent-drift risk).

Follow-ups: [[csl26-5397]] (suspected migrate title emph+quote artifact, ~68 styles), [[csl26-kohl]] (deferred analyzer improvements: savings ranking, array normalization, substitute/sort concerns, subsumption clustering).
