---
# csl26-ymm8
title: 'Spec: GivennameRule scope/form conflation cleanup'
status: todo
type: task
priority: normal
tags:
    - schema
    - chicago
    - disambiguation
    - contributors
created_at: 2026-08-14T14:17:43Z
updated_at: 2026-08-14T14:18:10Z
parent: csl26-40n4
---

GivennameRule has 5 variants (ByCite, AllNames, AllNamesWithInitials, PrimaryName,
PrimaryNameWithInitials) but the engine collapses them to 2 effective scopes (primary-only vs
all-names) — the *WithInitials suffixes are inert. Form (full vs initials) is always driven by
contributors.name-form, independent of GivennameRule. Confirmed by reading
crates/citum-engine/src/processor/disambiguation.rs (primary_givenname_only is computed by
matching GivennameRule::PrimaryName | PrimaryNameWithInitials as one arm — the initials
distinction is never read) and crates/citum-schema-style/src/options/contributors.rs (NameForm
enum + name_form field, doc comment already says "Initialization formatting details ...
are separate fields").

This is a schema change to citum-schema-style (removing or repurposing enum variants), so per
project policy it needs a docs/spec PR reviewed and merged before implementation. Surfaced
while fixing csl26-tc4x (Chicago author-date given-name disambiguation).

## Todo
- [ ] Write spec in docs/specs/ proposing either: (a) drop the two *WithInitials variants
      (breaking, needs migration path for any style using them), or (b) keep them but make
      the engine actually honor the form hint instead of always deferring to name-form
- [ ] Survey embedded styles for any current use of GivennameRule::*WithInitials to gauge
      blast radius
- [ ] Get spec reviewed/merged (status Draft -> Active) before any implementation PR
