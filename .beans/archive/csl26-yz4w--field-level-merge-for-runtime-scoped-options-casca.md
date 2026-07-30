---
# csl26-yz4w
title: Field-level merge for runtime scoped-options cascade
status: completed
type: feature
priority: normal
tags:
    - architecture
    - styles
    - schema
created_at: 2026-07-28T15:55:45Z
updated_at: 2026-07-30T11:44:44Z
parent: csl26-s2rw
---

The extends overlay now deep-merges nested option blocks (STYLE_INHERITANCE.md rule 1), but the runtime scope cascade (global -> citation/bibliography via Config::merge / merge_options! in crates/citum-schema-style/src/options/mod.rs) still replaces nested structs whole-value. Consequence: gb-t-7714-2025-base.yaml must keep a full bibliography.options.dates copy of its global dates block just to add note-wrap at bibliography scope. Design question: runtime merge has no raw YAML, so field-presence is ambiguous for defaulted non-Option fields (deserialized defaults are indistinguishable from authored defaults). Candidate approaches: presence-tracking wrapper types, keep raw per-scope mappings on Config, or an authored-value != default heuristic. Owned by UNIFIED_SCOPED_OPTIONS.md; STYLE_INHERITANCE.md deliberately scopes this out.

## Summary of Changes

Implemented the raw per-scope-mapping design (candidate 2). Each parsed style
captures its authored citation/bibliography `options` mappings (presets
expanded) in a new `Style.scoped_raw_options` field; `extends` resolution
chain-merges captures alongside the typed overlay. New
`CitationOptions::merged_with_raw` / `BibliographyOptions::merged_with_raw`
merge nested blocks field-by-field from the capture, guarded by the same
typed round-trip check as the overlay, falling back to the typed whole-block
merge (programmatic construction, post-parse mutation). Engine
`get_citation_config`/`get_bibliography_config` use the raw-aware path;
lint and citum-migrate SQI refinement intentionally stay typed.

Rejected alternatives: eager materialization at load time (freezes inherited
global fields into scope blocks, breaking propagation through wrapper
chains — covered by a regression test), presence-wrapper types (churn),
authored≠default heuristic (unsound).

Verified: gb-t-7714-2025-base's duplicated bibliography `dates` block
reduced to just `note-wrap`; full corpus render diff vs main (48 styles ×
bib+cite modes, direct `citum render refs`) is byte-identical. Spec:
UNIFIED_SCOPED_OPTIONS.md §2a; cross-ref in STYLE_INHERITANCE.md.
