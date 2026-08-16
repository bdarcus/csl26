---
# csl26-5753
title: Implement true by-cite per-position given-name expansion ceiling
status: todo
type: feature
priority: normal
tags:
    - engine
    - disambiguation
    - citation
created_at: 2026-08-16T18:30:22Z
updated_at: 2026-08-16T18:30:26Z
---

Follow-up from csl26-8nrt. docs/specs/DISAMBIGUATION.md §2.1.1 was corrected
2026-08-16: name disambiguation always compares against every reference in the
document, for all givenname-disambiguation-rule values, including by-cite. The
by-cite implementation that violated this (csl26-lvib, 2026-06-02) has been
removed. As a result, by-cite and all-names are now behaviorally identical in
Citum -- neither implements the escalation-cap semantics real by-cite has in
citeproc-js.

citeproc-js's actual by-cite behavior (scripts/node_modules/citeproc/citeproc_commonjs.js):
- Ambiguity detection is always registry-wide (CSL.Registry.ambigcites),
  regardless of givenname-disambiguation-rule.
- by-cite is rewritten to all-names for *position selection* purposes
  (`if (gdropt === "by-cite") { gdropt = "all-names"; }`).
- What actually varies is an escalation *ceiling*: `this.givensMax = 2` when
  by-cite + disambiguate-add-givenname are both active, plus a `request_base`
  floor check. This caps how far a single rendered cite is forced to expand --
  e.g. a cite showing two visible authors isn't forced to add given names for
  a third author hidden behind et-al in that same cite -- without narrowing
  which references are compared to detect the collision in the first place.

Citum's ProcHints model expresses given-name expansion as an all-or-nothing
flag (expand_given_names) plus a primary-only restriction
(expand_given_names_primary_only). There is no way to express "expand given
names for these specific rendered positions in this specific cite, capped at
N, per citeproc-js's request_base/givensMax logic" -- implementing true
by-cite requires that finer-grained model.

## Scope
- [ ] Design the per-position/per-cite expansion representation (likely an
      addition to ProcHints or a citation-render-time computation, not a
      citation-scoped rewrite of the global hint map -- that was the mistake
      csl26-8nrt corrected).
- [ ] Port citeproc-js's givensMax/request_base escalation-cap logic
      (CSL.Disambiguation.prototype.run and configModes, citeproc_commonjs.js
      ~L24415-24470 and ~L23640-23700).
- [ ] Add native fixtures that distinguish by-cite from all-names once the
      distinction is real again (the existing by_cite_scope_fixture tests were
      rewritten under csl26-8nrt to expect all-names-equivalent output; they
      will need re-splitting once this lands).
- [ ] Update docs/specs/DISAMBIGUATION.md §2.1.1 acceptance criteria.
