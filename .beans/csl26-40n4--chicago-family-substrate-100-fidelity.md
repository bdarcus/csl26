---
# csl26-40n4
title: Chicago family substrate & 100% fidelity
status: in-progress
type: epic
priority: high
created_at: 2026-06-30T14:29:06Z
updated_at: 2026-07-30T19:09:41Z
parent: csl26-w0hf
---

Container epic for the Chicago-family pivot: stop hand-tuning chicago-author-date-18th in isolation and instead build a shared semantic/component layer plus conversion/accessor fixes that lift chicago-author-date-18th, chicago-notes-18th, chicago-shortened-notes-bibliography, and taylor-and-francis-chicago-author-date together, backed by one robust fixture exercising citation+bibliography surfaces across all four, with each variant driven to ~100% fidelity.

## Architecture
Shared semantic/component layer, NOT one inherited bibliography template — rendered order stays per-style (author-date puts year after author; notes bibliography places date later; notes citations are a separate grammar).

1. Common Chicago 18 component policy/base: contributor formatting, title casing/quotes/italics, page-range format, DOI/URL conventions, periodical/book/chapter/media/archive component semantics, suppression policy (e.g. personal_communication bib suppression). Hidden base only where inheritance expresses commonality cleanly.
2. Two order layers kept separate: author-date reference-list ordering vs notes bibliography ordering.
3. Shared conversion/accessor facts (Rust, biggest cross-cutting win): archival correspondence, recordings, performances, broadcasts, original publication dates, event dates, note-derived roles.

## Current state (verified 2026-06-30)
Inheritance: chicago-author-date-18th -> book; chicago-notes-18th -> dataset (no shared base); chicago-shortened-notes-bibliography-core -> chicago-notes-18th; taylor-and-francis-chicago-author-date-core -> chicago-author-date-18th.

Fixtures fragmented: author-date variants use tests/fixtures/references-expanded.json; chicago-author-date-18th additionally carries tests/fixtures/test-items-library/chicago-18th.json (402 items, bibliography-only, min_pass_rate 0.73); chicago-notes-18th uses references-humanities-note.json, citation-only (scopes: [citation], no bibliography surface). No fixture exercises both surfaces across all four variants.

## Todo
- [ ] Chicago family audit doc (child bean, implemented same session)
- [ ] Common robust Chicago fixture wired into verification-policy.yaml for all four variants (child bean)
- [ ] Common Chicago 18 component policy/base (child bean)
- [ ] Chicago source-component conversion/accessor facts in Rust (child bean)
- [ ] Drive each variant to ~100% fidelity once substrate + fixture land (child bean)

Originating PR: https://github.com/citum/citum-core/pull/984 (merged as bae9d2fd, periodicals tuning) — this epic is the pivot away from further isolated tuning.
