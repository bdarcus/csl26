---
# csl26-boha
title: 'citum-migrate: emit locale message: calls for known Chicago phrase patterns'
status: todo
type: feature
priority: normal
tags:
    - migrate
    - localization
    - chicago
created_at: 2026-08-08T11:22:41Z
updated_at: 2026-08-08T11:22:50Z
---

Raw citum-migrate output hardcodes English prose for contributor-role phrases (e.g. 'Directed by', 'Translated by') instead of using Citum's locale message system, which is why PR #1151's cluster-1 localization pass had to hand-convert 25->1 and 12->1 hardcoded sites in chicago-author-date-18th and taylor-and-francis-chicago-author-date-core after the fact. The locale already ships eight pattern.chicago-* messages authored for this family (chicago-aired-date, chicago-by, chicago-interview-by, chicago-on, chicago-review-of, chicago-to, chicago-with, chicago-written-by) plus verb/verb-short forms on the relevant contributor roles, and scripts/style-structure-lint.js's STYLE010 rule already has the detection logic for hardcoded prose duplicating an existing locale term.

Idea: teach citum_migrate (crates/citum_migrate/src/) to recognize the same hardcoded-phrase patterns STYLE010 flags during conversion and emit message: pattern.chicago-* (or the general form: verb / form: verb-short role-label mechanism) directly, instead of a literal string prefix/text node. This makes any future or re-run migration come out already-localized, with no separate manual cluster-style pass needed afterward. Relevant specs: docs/specs/LOCALE_MESSAGES.md, docs/specs/CONTRIBUTOR_PHRASE_MESSAGES.md (Draft -- for the harder joint name/title/container-reordering case; this is scoped to the simpler compositional-phrase case PR #966 already solved, matching CHICAGO_FAMILY_STRATEGY.md's own scoping of cluster 1), docs/policies/LOCALIZATION_INTEGRITY.md.

Not scoped to Chicago specifically -- the same hardcoded-phrase problem likely recurs in any legacy CSL style with contributor-role text nodes, so the fix belongs in the shared migrate path, not a Chicago-only patch.

- [ ] Survey citum_migrate's current text/macro conversion path to find where a legacy <text value="..."/> or fixed prefix becomes a Citum Rendering/text component, and where a locale-message emission could hook in
- [ ] Reuse STYLE010's pattern list (or extract it to a shared location both the linter and migrate can reference) rather than re-deriving it
- [ ] Add migration test fixtures covering at least the eight existing pattern.chicago-* cases plus the general form:verb/verb-short path
- [ ] Verify against a re-run of chicago-author-date-18th/taylor-and-francis-chicago-author-date-core migration: STYLE010 hit count should already be low without a manual follow-up pass
- [ ] cargo nextest run, cargo clippy -D warnings
