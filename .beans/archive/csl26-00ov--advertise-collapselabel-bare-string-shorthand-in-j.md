---
# csl26-00ov
title: Advertise collapse/label bare-string shorthand in JSON schema
status: completed
type: task
priority: normal
tags:
    - schema
    - authorability
    - dx
created_at: 2026-08-31T17:54:01Z
updated_at: 2026-08-31T20:17:53Z
parent: csl26-h7oc
---

CitationCollapse has a hand-written Serialize/Deserialize
(crates/citum-schema-style/src/style/sections/citation.rs:47-109) that
accepts bare `same-author` as sugar for `{ same-author: {} }`, but only a
derived #[derive(JsonSchema)] -- schemars' default external tagging can't
represent a tuple variant as a bare string, so docs/schemas/style.json never
advertises the shorthand. This asymmetry is documented as deliberate in
docs/specs/SAME_AUTHOR_COLLAPSE.md, and is paralleled by
Processing::Label (crates/citum-schema-style/src/options/processing.rs) --
same gap, same reason, also derive-only; no hand-written JsonSchema impl
exists anywhere in the crate today.

Original complaint (user, csl26-lr1p FIX annotation on
chicago-author-date-18th.yaml): `same-author: {}` is bad YAML surface for
style authors. A prior attempt in this session to fix it by switching
embedded styles to the bare-string shorthand broke CI (schema-hygiene check
validates against the published, shorthand-blind schema) and was reverted --
net progress on this complaint from that attempt: zero.

The fix should cover CitationCollapse alone, and Processing::Label
(the documented parallel case)?

## Summary of Changes

Added schema-visible scalar shorthands for citation collapse and processing, added `plain` title rendering, removed literal empty objects from tracked styles, and added fatal STYLE012 lint coverage. Updated the same-author and typed-title documentation, regenerated the style schema, and verified identical rendering for every changed style plus the full core quality floor.
