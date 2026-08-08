---
# csl26-zl7f
title: |-
    chicago-shortened-notes-bibliography-core: bibliography separator uses comma instead of period  (chicago-shortened-notes-bibliography-core.yaml:92) where every other Chicago bibliography style (e.g. chicago-author-date-18th.yaml:193) uses . . This joins every top-level bibliography component with a comma instead of a period across the whole entry -- e.g. oracle 'Chen, Wei. "Urban Heat Island Mitigation Strategies." Urban Climate 15 (2022): 1-18.' renders in Citum as 'Chen, Wei, "Urban Heat Island Mitigation Strategies," Urban Climate, 15, 2022: 1-18,' -- wrong throughout, not at one boundary.

    Evidence (node scripts/report-core.js, references-expanded + chicago-18th corpora, 2026-08-08): of 422 currently-failing exactParity bibliography entries for chicago-shortened-notes-bibliography, 326 (77%) match this exact signature (citum text has at most one internal '. ' where oracle has two or more). This is almost certainly the single largest contributor to the style's 13/473 (2.7%) exact-parity score -- far larger than any of the shared defect-cluster work in csl26-h7oc's children, and specific to this one style/one option.

    Likely fix: change bibliography.options.separator from ', ' to '. ' in chicago-shortened-notes-bibliography-core.yaml. One-line change; per project policy style YAML changes skip the Rust pre-commit gate, but still need citeproc-js verification before landing (see csl26-hv1g/csl26-j1wp precedent -- verify against node scripts/report-core.js, don't assume from the signature alone that every one of the 326 clears, since some entries will have other independent defects layered on top).

    - [ ] Confirm the fix against a handful of the 326 flagged entries individually, not just the aggregate count
    - [ ] node scripts/report-core.js before/after on chicago-shortened-notes-bibliography specifically
    - [ ] cargo nextest run
    - [ ] Check whether this also affects chicago-shortened-notes-bibliography.yaml's variant siblings (16th/17th-edition, archive-place-first, etc.) that extend -core
    - [ ] Note the outcome in this bean's summary when done
status: todo
type: bug
priority: critical
tags:
    - style
    - fidelity
    - chicago
created_at: 2026-08-08T13:35:24Z
updated_at: 2026-08-08T13:35:55Z
parent: csl26-h7oc
---

bibliography.options.separator is ", " (chicago-shortened-notes-bibliography-core.yaml:92) where every other Chicago bibliography style (e.g. chicago-author-date-18th.yaml:193) uses ". ". This joins every top-level bibliography component with a comma instead of a period across the whole entry -- e.g. oracle 'Chen, Wei. "Urban Heat Island Mitigation Strategies." Urban Climate 15 (2022): 1-18.' renders in Citum as 'Chen, Wei, "Urban Heat Island Mitigation Strategies," Urban Climate, 15, 2022: 1-18,' -- wrong throughout, not at one boundary.

Evidence (node scripts/report-core.js, references-expanded + chicago-18th corpora, 2026-08-08): of 422 currently-failing exactParity bibliography entries for chicago-shortened-notes-bibliography, 326 (77%) match this exact signature (citum text has at most one internal '. ' where oracle has two or more). This is almost certainly the single largest contributor to the style's 13/473 (2.7%) exact-parity score -- far larger than any of the shared defect-cluster work in csl26-h7oc's children, and specific to this one style/one option.

Likely fix: change bibliography.options.separator from ', ' to '. ' in chicago-shortened-notes-bibliography-core.yaml. One-line change; per project policy style YAML changes skip the Rust pre-commit gate, but still need citeproc-js verification before landing (see csl26-j1wp -- verify against node scripts/report-core.js, don't assume from the signature alone that every one of the 326 clears, since some entries will have other independent defects layered on top).

- [ ] Confirm the fix against a handful of the 326 flagged entries individually, not just the aggregate count
- [ ] node scripts/report-core.js before/after on chicago-shortened-notes-bibliography specifically
- [ ] cargo nextest run
- [ ] Check whether this also affects chicago-shortened-notes-bibliography.yaml's variant siblings (16th/17th-edition, archive-place-first, etc.) that extend -core
- [ ] Note the outcome in this bean's summary when done
