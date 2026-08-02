---
# csl26-y49d
title: 'ieee missing type-variants: legal_case, treaty, government-act, apparatus'
status: todo
type: task
priority: normal
tags:
    - migrate
    - style
    - fidelity
created_at: 2026-08-02T00:09:16Z
updated_at: 2026-08-02T13:16:20Z
---

Hand-author the missing ieee bibliography type-variants: legal_case, treaty, government-act (bill/legislation), and apparatus/equipment. Right now they fall back to the generic monograph template, which gets title quoting, container title, and field order all wrong for these types.

- The source (ieee.csl) has real, distinct rules for each -- this is copying them over, not designing new behavior.
- Examples currently wrong: "Brown v. Board of Education", "Every Student Succeeds Act", the Tactile Labs apparatus entry.
- Likely the single biggest remaining piece of ieee's exact-parity gap.
- Before starting: check whether other numeric/engineering CSL styles need the same branches -- worth building once if so, not per-style each time.
