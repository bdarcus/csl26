---
# csl26-yz4w
title: Field-level merge for runtime scoped-options cascade
status: todo
type: feature
priority: normal
tags:
    - architecture
    - styles
    - schema
created_at: 2026-07-28T15:55:45Z
updated_at: 2026-07-28T15:55:45Z
parent: csl26-s2rw
---

The extends overlay now deep-merges nested option blocks (STYLE_INHERITANCE.md rule 1), but the runtime scope cascade (global -> citation/bibliography via Config::merge / merge_options! in crates/citum-schema-style/src/options/mod.rs) still replaces nested structs whole-value. Consequence: gb-t-7714-2025-base.yaml must keep a full bibliography.options.dates copy of its global dates block just to add note-wrap at bibliography scope. Design question: runtime merge has no raw YAML, so field-presence is ambiguous for defaulted non-Option fields (deserialized defaults are indistinguishable from authored defaults). Candidate approaches: presence-tracking wrapper types, keep raw per-scope mappings on Config, or an authored-value != default heuristic. Owned by UNIFIED_SCOPED_OPTIONS.md; STYLE_INHERITANCE.md deliberately scopes this out.
