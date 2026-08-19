---
# csl26-ax22
title: 'Chicago notes: locator punctuation wrong on single-item citations'
status: todo
type: bug
priority: normal
tags:
    - note-styles
    - chicago
    - rendering
    - citation
created_at: 2026-08-19T13:48:54Z
updated_at: 2026-08-19T13:49:02Z
parent: csl26-h7oc
---

`chicago-notes-18th` (and likely siblings sharing its template) renders `4 (2019), 257` for a citation with a page locator, where citeproc-js/CMOS renders `4 (2019): 257` (colon before the locator, not comma). Reproduces on a plain single-item citation — independent of same-author collapse.

Noticed while investigating `csl26-m11m` / `csl26-ecfn`: the second clause of a same-author collapsed cluster with a locator on the second item (`[@ITEM-31; @ITEM-32, p. 257]`) shows this exact defect and is pinned as such — not asserted as oracle parity — in `given_chicago_notes_style_when_same_author_cluster_has_a_locator_then_each_item_renders_in_full` (`crates/citum-engine/tests/domain_fixtures.rs`). Deliberately out of scope for that PR (`docs/specs/SAME_AUTHOR_COLLAPSE.md` Scope section).

## Scope
- [ ] Reproduce on a single-item `chicago-notes-18th` citation with a page locator to confirm this is unrelated to grouping/collapse.
- [ ] Find the template/punctuation rule that should route the locator after a colon rather than a comma for this style's citation template.
- [ ] Add a regression test pinning the fix against the citeproc-js oracle.
- [ ] Re-run `report-core.js --style chicago-notes-18th` to measure exactParity movement.
