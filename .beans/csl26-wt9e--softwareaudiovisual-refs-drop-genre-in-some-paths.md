---
# csl26-wt9e
title: Software/AudioVisual refs drop genre in some paths
status: todo
type: bug
priority: normal
tags:
    - schema
    - fidelity
created_at: 2026-09-06T15:56:37Z
updated_at: 2026-09-06T15:56:37Z
parent: csl26-ccdt
---

InputReference::Software (crates/citum-schema-data/src/reference/
conversion/media.rs, from_software_ref) has no `genre` field at all --
`legacy.genre` is read for nothing; only `legacy.medium` survives (as
`platform`). Any style whose template renders `variable: genre` for a
software-type reference gets nothing back, regardless of style config.

Repro: apa-7th and modern-language-association both render
TLIB-SEL-SOFTWARE-1 (type: software, genre: "Model", medium:
"Apparatus") as "[Apparatus]" -- medium survives, genre is silently
dropped. Oracle wants "[Model; Apparatus]" (APA) / "Model. ... .
Apparatus." (MLA, genre and medium in different template slots).

By contrast, the from_monograph_ref-family conversion path
(crates/citum-schema-data/src/reference/conversion/scholarly.rs:947-956)
correctly seeds `genre` from `legacy.genre.clone()` (with a documented
ref_type fallback for map/figure/graphic/periodical/collection). Map
type (TLIB-SEL-MAP-1, genre: "Map") goes through this path and its
genre field IS present -- but its rendering ALSO drops the bracket in
APA's base template despite the field being populated, which needs
separate investigation (possibly a rendering-side issue, not a
conversion-side one -- didn't get to trace this one before time ran
out on this pass).

## Scope
- Software: add a `genre` field to the `Software` struct
  (crates/citum-schema-data/src/reference/types/... wherever Software
  is defined) and thread it through from_software_ref, matching the
  AudioVisualWork pattern already used at media.rs:70. Schema-touching
  (regenerate crates/citum-schema-data conversion docs per CLAUDE.md).
- Map: re-trace why a populated `genre` field still doesn't reach APA's
  `variable: genre` component for this type -- possibly unrelated to
  Software's missing-field bug.
Found tuning csl26-7e6l (apa-7th/modern-language-association second-pass
sweep); not attempted in that PR since it's Rust/schema-data, not YAML.
