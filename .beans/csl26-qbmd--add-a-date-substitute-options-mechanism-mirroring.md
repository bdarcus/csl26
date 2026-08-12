---
# csl26-qbmd
title: Add a date-substitute options mechanism mirroring author-substitute
status: todo
type: feature
priority: normal
tags:
    - engine
    - fidelity
    - gb-t-7714
created_at: 2026-08-12T13:59:09Z
updated_at: 2026-08-12T13:59:09Z
parent: csl26-ccdt
---

GB/T 7714's date-fallback chains today live inline, duplicated across many `TemplateDate` components across many type-variants — each spelling out its own `fallback:` list (e.g. `book,thesis,map`'s `[date: copyright, date: printing, date: accessed, message: term.no-date]`, `webpage,post,post-weblog`'s accessed-year fallback, `article-journal,article-magazine`'s empty `fallback: []`). This is structurally the same shape `options/substitute.rs`'s `Substitute`/`SubstituteConfig` already generalizes for contributor (author) fallback — a single, reusable, type-scoped (`overrides: HashMap<String, Vec<SubstituteKey>>`) declarative chain for one well-defined slot, resolved per-reference-type via `values/contributor/substitute.rs`.

Raised during csl26-huuz's review: rather than growing `TemplateDate`'s per-component `suppress-*` flag pile (`suppress-note`, `suppress-disamb-suffix`, and a since-reverted `suppress-no-date-term`) to express 'nothing else to show when the date is missing', add a `date-substitute` options mechanism mirroring `author-substitute`.

Scope decision needed: contributor substitution has exactly one addressable slot (the author position). `TemplateDate` components appear in many distinct structural roles (issued-date position, accessed footer, original-published in a reprint template). The natural scope is the *primary/identity* date slot only — the one `first_date_component_for_bibliography`/`_for_citation` (crates/citum-engine/src/sorting.rs) already isolate for disambiguation purposes — not every `TemplateDate` occurrence.

Per Bruce: this must be designed together with the disambiguation collision-key discriminant (csl26-huuz, PR #1171), not built independently and reconciled after — almost all of what `Disambiguator::date_component_discriminant` reads (message candidates, date candidates, the accessed-date no-identity rule, the empty/blank case) is exactly the territory this mechanism will own. Structure: stacked PRs off #1171 (gh stack) — spec (Draft status) first, then implementation (Active status) migrating GB/T's inline fallback chains and rewiring the discriminant/renderer to consume the resolved date-substitute output instead of `TemplateDate.fallback`. See docs/specs/DISAMBIGUATION.md and csl26-sea6 (sibling gap in the same style).
