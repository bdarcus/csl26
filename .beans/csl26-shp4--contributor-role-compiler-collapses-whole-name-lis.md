---
# csl26-shp4
title: Contributor role compiler collapses whole name list (maps to Composer not Compiler)
status: todo
type: bug
priority: high
tags:
    - style
    - chicago
    - fidelity
    - contributors
created_at: 2026-09-04T13:02:52Z
updated_at: 2026-09-04T13:03:07Z
parent: csl26-h7oc
---

raw_conversion.rs parse_role_name maps CSL-legacy role key compiler to ContributorRole::Composer instead of the already-existing ContributorRole::Compiler variant (contributor.rs:210). No chicago-* type-variant contributor:author selector treats Composer as eligible for the primary slot, so the sole contributor silently disappears entirely (name dropped, title promoted sentence-initial). 6 confirmed rows across 3 styles; small row count but a contributor-dropping correctness bug. See plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md

Related: csl26-i7nz (contributor role and ordering grammar) covers the broader class; this is its 'unsupported primary-contributor roles collapsing the whole name list' sub-case called out in that bean's body.
