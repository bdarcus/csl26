---
# csl26-57a7
title: Audit TemplateComponent::Group consumers for an Alternatives arm
status: todo
type: task
priority: normal
tags:
    - engine
    - style
    - schema
    - fidelity
created_at: 2026-09-07T00:42:55Z
updated_at: 2026-09-07T00:43:01Z
parent: csl26-40n4
---

docs/specs/ALTERNATIVES.md (Draft, csl26-la9t's third review round) proposes a new TemplateComponent::Alternatives variant. A grep found TemplateComponent::Group matched in 21 files across citum-engine and citum-schema-style -- not just the renderer. Three were checked directly and confirmed to key on specific component kinds (Title, Date(Issued), Number(Volume), Variable(Url/Doi)) for non-rendering purposes:

- crates/citum-engine/src/values/list.rs (is_term_based -- already accounted for in the spec)
- crates/citum-engine/src/processor/rendering/grouped/component_predicates.rs (citation grouping / contributor-stripping)
- crates/citum-engine/src/processor/rendering/grouped/template_policy.rs (article-journal bibliography template filtering, the same file csl26-8z39 extends)

If a title, contributor, date, number, or url/doi variable is wrapped inside an alternatives: candidate, these consumers won't see it structurally, regardless of nesting depth -- restricting nesting does not fix this, since the cause is what a candidate contains, not how deep it sits.

The remaining 18 files matching TemplateComponent::Group were not audited this session. ALTERNATIVES.md v1 restricts alternatives: to positions none of the 3 checked consumers touch (not the primary title/contributor position, not inside an article-journal type-variant's top-level bibliography template, no nesting) as an interim measure.

## Todo
- [ ] Enumerate the remaining 18 files matching TemplateComponent::Group (grep -rln 'TemplateComponent::Group' crates/citum-engine/src crates/citum-schema-style/src, minus the 3 already checked)
- [ ] For each, determine whether it needs an Alternatives arm to remain correct once alternatives: is in general use
- [ ] Add the arm to each file that needs one, with a regression test per file
- [ ] Once complete, lift ALTERNATIVES.md's v1 placement restriction (update Scope/'v1 placement restriction' section, promote to v2)
