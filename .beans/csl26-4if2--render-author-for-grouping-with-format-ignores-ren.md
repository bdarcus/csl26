---
# csl26-4if2
title: render_author_for_grouping_with_format ignores render-when on the leading template component
status: todo
type: bug
priority: normal
tags:
    - engine
    - citation
created_at: 2026-08-31T12:24:41Z
updated_at: 2026-08-31T17:53:28Z
parent: csl26-h7oc
---

crates/citum-engine/src/processor/rendering/grouped/core.rs render_author_for_grouping_with_format (~lines 840-908) resolves the citation author-grouping slot from template.first() unconditionally, ignoring that component's render-when condition, then falls back to reference.author() regardless. Surfaced while trialling a render-when-gated author/publisher split for chicago-author-date-18th's webpage citation (see csl26-lr1p / csl26-f3hx): an author-group-first split rendered both the title AND the publisher for an authorless reference instead of just the publisher, because the fallback ran even though the author component's own render-when should have suppressed it. Separate fix surface from the SubstituteField::Publisher work in csl26-f3hx.
