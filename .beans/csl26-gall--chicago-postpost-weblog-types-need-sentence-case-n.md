---
# csl26-gall
title: 'Chicago: post/post-weblog types need sentence-case, not title-case'
status: todo
type: bug
priority: normal
tags:
    - style
    - chicago
    - title
    - fidelity
created_at: 2026-08-24T11:50:28Z
updated_at: 2026-08-24T11:50:28Z
parent: csl26-h7oc
---

citeproc-js's real title choose-block routes post/post-weblog to 'quotes, sentence case' -- Citum currently title-cases these (same category-routing shape as the document/thesis fix landed in the quote-boundary wave, PR #1226, via titles.type-mapping). Discovered via a stop-word-gap false positive: an Instagram post-type item ('9/11 Anniversaries from the Past' vs oracle '9/11 anniversaries from the past') looked like a missing stop word ('past') but the whole title should be sentence case, not title case at all. Likely a fast wave-3-adjacent fix once triaged: map post/post-weblog to a category with no title-case transform (or add an explicit type-variant), verified per-entry against the oracle.
