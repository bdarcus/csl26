---
# csl26-ckcf
title: 'chicago-author-date-18th: manuscript/collection bibliography renders archive-collection twice'
status: todo
type: bug
priority: low
tags:
    - style
    - chicago
    - fidelity
created_at: 2026-08-26T14:51:02Z
updated_at: 2026-08-26T14:51:05Z
parent: csl26-h7oc
---

manuscript/collection bibliography entries with an archive-collection value render it twice consecutively, e.g. '...George Washington Papers, Series 5: Financial Papers, 1750-96. George Washington Papers, Series 5: Financial Papers, 1750-96. Library of Congress...' (fixture 6188419/YVLJ984N). Real Chicago (oracle) renders it once. Pre-existing, unrelated to csl26-0u0f's substitute-title-formatting work (confirmed via before/after rawCitum diff on the same fixture rows -- the duplication is present in both). Likely a duplicate archive-collection/variable component in the manuscript and/or collection bibliography type-variant in chicago-author-date-18th.yaml (crates/citum-schema-style/embedded/styles/chicago-author-date-18th.yaml, ~line 670).
