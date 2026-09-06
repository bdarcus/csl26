---
# csl26-zs9y
title: Style templates can't condition a literal on URL presence
status: todo
type: task
priority: normal
tags:
    - schema
    - fidelity
    - style
created_at: 2026-09-05T21:24:36Z
updated_at: 2026-09-06T15:27:08Z
parent: csl26-ccdt
---

NLM-family styles (T&F-NLM, and IEEE has a similar case) need a literal
"[Internet]" marker appended to the TITLE component, but only for
entries that have a URL (i.e. are online-only sources) -- and
separately "Available from: " as the URL's own prefix, and "[cited
DATE]" attached to the accessed date. The latter two are already free
(a `variable: url` or `date: accessed` component auto-suppresses when
its own field is empty), but the title-suffix marker has no field of
its own to piggyback on.

The only field-presence gate in the schema is `TemplateGroup.render_when:
TemplateGroupCondition` (crates/citum-schema-style/src/template.rs:1790-1816),
and it only exists on `group` components -- not on `title`,
`variable`, or any other leaf component. Its `TemplateConditionField`
enum (same file, ~1822-1870) also has no `Url`/`Doi`-presence variant
usable for this (it has `Doi` but not `Url`).

Found tuning csl26-on47 (T&F-NLM: ~8 residual rows across webpage,
dataset, interview, map, hearing, software types all need this same
"[Internet]. ... Available from: URL." pattern; IEEE has an analogous
"[Online]. Available: URL" pattern for similar types). Both are
blocked on the same missing primitive.

Per docs/guides (schema changes need a docs-first spec before
implementation): this needs a spec in docs/specs/ proposing either (a)
a `Url` variant added to `TemplateConditionField` plus wiring
`render_when` (or an equivalent) onto `TemplateTitle` and other leaf
components, or (b) a narrower purpose-built "online-source marker"
component. Scope: engine (`crates/citum-engine`) + schema
(`crates/citum-schema-style`), not style YAML.
