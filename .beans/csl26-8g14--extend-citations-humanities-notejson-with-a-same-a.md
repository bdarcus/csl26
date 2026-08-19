---
# csl26-8g14
title: Extend citations-humanities-note.json with a same-author cluster
status: todo
type: task
priority: low
tags:
    - note-styles
    - citation
    - rendering
created_at: 2026-08-19T13:48:34Z
updated_at: 2026-08-19T13:48:42Z
---

`docs/specs/SAME_AUTHOR_COLLAPSE.md`'s note-regime fix (`csl26-m11m`) is currently pinned only in `crates/citum-engine/tests/domain_fixtures.rs`, using `references-expanded.json`'s ITEM-31/ITEM-32 (Garcia) via native `InputReference` construction — not the dedicated humanities-note oracle-snapshot fixture family.

`tests/fixtures/citations-humanities-note.json` / `references-humanities-note.json` would be a more natural home for a same-author, note-regime cluster (archival/manuscript-heavy references, matching that fixture's domain), but extending it regenerates the `note-humanities` oracle snapshot family — its own blast-radius check, confirmed as a separate change per `docs/guides/shared-fixture-blast-radius-check.md`'s precedent (2 entries in `references-expanded.json` once regenerated 2,845 oracle snapshots).

## Scope
- [ ] Add a same-author multi-item cluster to `citations-humanities-note.json` with matching references in `references-humanities-note.json`.
- [ ] Check blast radius before committing (how many oracle snapshots regenerate) — prefer ad-hoc verification over a permanent fixture commit if the radius is large, per repo precedent.
- [ ] Regenerate and commit the affected `tests/snapshots/csl/*.json` files if the fixture change is kept.

See `docs/specs/SAME_AUTHOR_COLLAPSE.md` Scope section ("Extending tests/fixtures/citations-humanities-note.json ... a separate change with its own oracle-snapshot blast radius").
