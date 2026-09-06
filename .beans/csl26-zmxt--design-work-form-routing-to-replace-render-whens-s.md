---
# csl26-zmxt
title: Design work-form routing to replace render-when's structural-policy uses
status: todo
type: task
priority: normal
tags:
    - schema
    - engine
    - style
    - chicago
    - fidelity
created_at: 2026-09-06T21:58:21Z
updated_at: 2026-09-06T21:58:26Z
parent: csl26-40n4
---

Under csl26-40n4 (Chicago family substrate). The render-when disposition audit (docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md, csl26-h8ja) found 25 of render-when's 125 uses are structural policy gates -- the tested field (volume-or-issue, part-number-numeric, part-number-non-numeric, genre, title) never appears in the branch it guards, meaning it routes an unrelated component (editor form, container title, page prefix) based on a property of the reference. No declarative primitive covers this today; it's why render-when can't be removed, only frozen.

Forcing-case inventory (file:line in the audit's appendix): chicago-author-date-18th.yaml:457/465 (editor form by volume-or-issue), chicago-author-date-18th.yaml:426/489 (title routing by part-number-non-numeric), and the genre/title B-shape uses across chicago-notes-18th.yaml and chicago-shortened-notes-bibliography-core.yaml.

Likely home: docs/specs/INPUT_REFERENCE_CLASS_DISCRIMINATOR.md, or a new work-form concept alongside it. Related to csl26-x61x (Chicago volume/issue/series grammar).

## Todo
- [ ] Enumerate the full 25-use B-shape set with rendered-content diffs (what actually differs between branches)
- [ ] Propose a declarative primitive (option, discriminator, or type-variant axis)
- [ ] Spec in docs/specs/ before implementation
- [ ] Once shipped, migrate render-when's structural-policy uses and deprecate render-when
