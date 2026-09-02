---
# csl26-4if2
title: render_author_for_grouping_with_format ignores render-when on the leading template component
status: completed
type: bug
priority: normal
tags:
    - engine
    - citation
created_at: 2026-08-31T12:24:41Z
updated_at: 2026-09-02T18:19:50Z
parent: csl26-h7oc
---

crates/citum-engine/src/processor/rendering/grouped/core.rs render_author_for_grouping_with_format (~lines 840-908) resolves the citation author-grouping slot from template.first() unconditionally, ignoring that component's render-when condition, then falls back to reference.author() regardless. Surfaced while trialling a render-when-gated author/publisher split for chicago-author-date-18th's webpage citation (see csl26-lr1p / csl26-f3hx): an author-group-first split rendered both the title AND the publisher for an authorless reference instead of just the publisher, because the fallback ran even though the author component's own render-when should have suppressed it. Separate fix surface from the SubstituteField::Publisher work in csl26-f3hx.

## Summary of Changes

`render_author_for_grouping_with_format` (crates/citum-engine/src/processor/
rendering/grouped/core.rs) resolved the leading citation template component
via `template.first().and_then(find_grouping_component)`, but
`find_grouping_component` descends *into* a `Group`'s children without ever
checking the Group's own `render_when` — so a render-when-gated leading
author group had its condition silently discarded, and the unconditional
`reference.author()` fallback then ran regardless.

Fix: check `template.first()`'s `render_when` (when it is a `Group`) before
doing anything else. If the condition evaluates false, return an empty
string immediately — no fallback — mirroring the render_when-aware walk
`find_template_title_node` (values/contributor/substitute.rs) already uses
for the analogous title-lookup case, via the same
`crate::values::group_condition_matches` helper.

Added `grouped_citation_honors_render_when_on_leading_author_group`
(crates/citum-engine/tests/citations.rs), an `#[rstest]` with 2 cases built
from a native `InputReference` (no CSL-JSON round trip) and a minimal custom
citation-template YAML: an authored reference renders "Smith 2020" as
before; an authorless reference renders bare "2020" — the author slot stays
empty rather than falling back.

No embedded style currently puts `render-when` on a leading citation
component, so this produces zero snapshot diff (confirmed by the full
`just pre-commit` gate passing unchanged).

## Post-merge CI finding (2026-09-02)

PR #1251's Fidelity Checks caught a real, net-positive side effect on
chicago-shortened-notes-bibliography: honoring render-when correctly (rather
than ignoring it) changed this style's exact-parity corpus count from 87 to
86 (~21 items gained, 3 lost). One of the 3 losses is an editor-only
reference whose author+title citation group is gated on `render-when:
field-present: author` -- the old buggy code accidentally rendered it
correctly because contributor substitution (editor standing in for author)
applied independently of the (ignored) gate. Filed csl26-x79y to track that
separate gap (render-when field-presence checks the raw field, not the
substitution-aware effective contributor). Updated
scripts/report-core.test.js's hardcoded exact-parity count to 86 with a
justifying comment, following the test's own established precedent
(81 -> 86 -> 87 -> 86).
