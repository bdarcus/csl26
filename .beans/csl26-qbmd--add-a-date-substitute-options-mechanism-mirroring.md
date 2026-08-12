---
# csl26-qbmd
title: Add a date-substitute options mechanism mirroring author-substitute
status: in-progress
type: feature
priority: normal
tags:
    - engine
    - fidelity
    - gb-t-7714
created_at: 2026-08-12T13:59:09Z
updated_at: 2026-08-12T15:53:10Z
parent: csl26-ccdt
---

GB/T 7714's date fallback is a style-family policy currently repeated inline
across identity and display-only `TemplateDate` components. The public contract
is specified in `docs/specs/DATE_SUBSTITUTE.md`.

The accepted v1 design provides three presets (`standard`,
`gb-t-7714-2025`, and `gb-t-7714-2025-author-date`) plus a flat ordered
`TypeSelector` map escape hatch. Omission preserves inline or implicit behavior;
explicit `standard` selects the standard options-level policy; a matched empty
list intentionally renders the identity slot blank.

Rendering and disambiguation must consume the same tri-state resolved candidate
source and effective scope options. The mechanism applies only to the first date
component whose `suppress-disamb-suffix` is not true; later and display-only
dates keep inline fallback.

Four-layer stacked PR:

1. PR #1171 — candidate-neutral date-slot foundation.
2. `docs/specs/DATE_SUBSTITUTE.md` — Draft specification.
3. Schema and engine implementation; spec becomes Active.
4. GB/T style migration to the two GB/T presets with fidelity validation.

Related: `docs/specs/DISAMBIGUATION.md`, csl26-sea6.
